//! A TrueType subsetter for embedding the bundled fonts in SVG output.
//!
//! It keeps the glyphs for a set of characters, plus `.notdef` and composite
//! components, renumbers them, and writes a font with a fresh `cmap`, `hmtx`,
//! `loca` and `glyf`. Layout tables (GSUB, GPOS, GDEF) are dropped because
//! measurement uses unkerned advances and SVG output disables features. The
//! `name` table is rewritten under the output family: the OFL reserves the
//! upstream name for modified versions. Only BMP characters are mapped.

use std::collections::{BTreeMap, BTreeSet};

use ttf_parser::{Face, RawFace, Tag};

/// Tables copied unchanged. Hinting tables stay so hinted rasterizers can
/// run the glyph programs.
const COPIED: [&[u8; 4]; 5] = [b"OS/2", b"cvt ", b"fpgm", b"prep", b"gasp"];

/// Name IDs copied from the source font: copyright, version, license, license URL.
const COPIED_NAMES: [u16; 4] = [0, 5, 13, 14];

/// Returns the font and the characters it maps.
pub(super) fn subset(
    data: &[u8],
    chars: &BTreeSet<char>,
    family: &str,
    style: &str,
) -> (Vec<u8>, Vec<u16>) {
    let face = Face::parse(data, 0).expect("bundled font must be valid");
    let raw = RawFace::parse(data, 0).expect("bundled font must be valid");
    let table = |tag: &[u8; 4]| {
        raw.table(Tag::from_bytes(tag))
            .unwrap_or_else(|| panic!("bundled font lacks {}", String::from_utf8_lossy(tag)))
    };
    let (head, hhea, maxp, hmtx, loca, glyf) = (
        table(b"head"),
        table(b"hhea"),
        table(b"maxp"),
        table(b"hmtx"),
        table(b"loca"),
        table(b"glyf"),
    );
    let glyph = |gid: u16| {
        let at = |i: usize| {
            if u16_at(head, 50) == 0 {
                u16_at(loca, i * 2) as usize * 2
            } else {
                u32_at(loca, i * 4) as usize
            }
        };
        &glyf[at(gid as usize)..at(gid as usize + 1)]
    };

    // Keep .notdef, the mapped glyphs, and every composite's components.
    let mapped: BTreeMap<u16, u16> = chars
        .iter()
        .filter(|&&c| (c as u32) < 0x10000)
        .filter_map(|&c| Some((c as u16, face.glyph_index(c)?.0)))
        .collect();
    let mut keep: BTreeSet<u16> = mapped.values().copied().chain([0]).collect();
    let mut todo: Vec<u16> = keep.iter().copied().collect();
    while let Some(gid) = todo.pop() {
        for (_, component) in components(glyph(gid)) {
            if keep.insert(component) {
                todo.push(component);
            }
        }
    }
    let new_id: BTreeMap<u16, u16> = keep
        .iter()
        .enumerate()
        .map(|(i, &g)| (g, i as u16))
        .collect();

    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::new();
    let mut new_hmtx = Vec::new();
    let metrics = u16_at(hhea, 34) as usize;
    for &gid in &keep {
        new_loca.extend((new_glyf.len() as u32).to_be_bytes());
        let mut g = glyph(gid).to_vec();
        for (at, component) in components(glyph(gid)) {
            g[at..at + 2].copy_from_slice(&new_id[&component].to_be_bytes());
        }
        new_glyf.extend(&g);
        new_glyf.resize(new_glyf.len().next_multiple_of(4), 0);
        let i = gid as usize;
        let advance = u16_at(hmtx, i.min(metrics - 1) * 4);
        let lsb = if i < metrics {
            u16_at(hmtx, i * 4 + 2)
        } else {
            u16_at(hmtx, metrics * 4 + (i - metrics) * 2)
        };
        new_hmtx.extend(advance.to_be_bytes());
        new_hmtx.extend(lsb.to_be_bytes());
    }
    new_loca.extend((new_glyf.len() as u32).to_be_bytes());

    let count = (keep.len() as u16).to_be_bytes();
    let mut new_head = head.to_vec();
    new_head[8..12].fill(0); // checkSumAdjustment, set once the file is assembled
    new_head[50..52].copy_from_slice(&1u16.to_be_bytes()); // long loca offsets
    let mut new_hhea = hhea.to_vec();
    new_hhea[34..36].copy_from_slice(&count);
    let mut new_maxp = maxp.to_vec();
    new_maxp[4..6].copy_from_slice(&count);
    // post format 3: the header without glyph names.
    let mut post = table(b"post")[..32].to_vec();
    post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());

    let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"head", new_head),
        (*b"hhea", new_hhea),
        (*b"maxp", new_maxp),
        (*b"hmtx", new_hmtx),
        (*b"loca", new_loca),
        (*b"glyf", new_glyf),
        (*b"cmap", cmap(mapped.iter().map(|(&c, g)| (c, new_id[g])))),
        (*b"post", post),
        (*b"name", name(table(b"name"), family, style)),
    ];
    tables.extend(
        COPIED
            .iter()
            .filter_map(|tag| Some((**tag, raw.table(Tag::from_bytes(tag))?.to_vec()))),
    );
    (sfnt(tables), mapped.into_keys().collect())
}

/// Component glyph IDs of a composite glyph, with the byte offset of each ID.
fn components(g: &[u8]) -> Vec<(usize, u16)> {
    let mut out = Vec::new();
    if g.len() < 10 || (u16_at(g, 0) as i16) >= 0 {
        return out;
    }
    let mut at = 10;
    loop {
        let flags = u16_at(g, at);
        out.push((at + 2, u16_at(g, at + 2)));
        at += 4 + if flags & 0x0001 != 0 { 4 } else { 2 };
        at += match flags {
            f if f & 0x0008 != 0 => 2,
            f if f & 0x0040 != 0 => 4,
            f if f & 0x0080 != 0 => 8,
            _ => 0,
        };
        if flags & 0x0020 == 0 {
            return out;
        }
    }
}

/// A Windows Unicode BMP `cmap` with one format 4 segment per character.
fn cmap(map: impl Iterator<Item = (u16, u16)>) -> Vec<u8> {
    let mut segments: Vec<(u16, u16)> = map.map(|(c, g)| (c, g.wrapping_sub(c))).collect();
    segments.push((0xffff, 1));
    let n = segments.len() as u16;
    let mut sub = Vec::new();
    for v in [4, 16 + 8 * n, 0, 2 * n] {
        sub.extend(v.to_be_bytes());
    }
    sub.extend(search_fields(n, 2));
    segments
        .iter()
        .for_each(|(c, _)| sub.extend(c.to_be_bytes()));
    sub.extend(0u16.to_be_bytes());
    segments
        .iter()
        .for_each(|(c, _)| sub.extend(c.to_be_bytes()));
    segments
        .iter()
        .for_each(|(_, d)| sub.extend(d.to_be_bytes()));
    segments.iter().for_each(|_| sub.extend(0u16.to_be_bytes()));

    let mut out = Vec::new();
    for v in [0u16, 1, 3, 1] {
        out.extend(v.to_be_bytes());
    }
    out.extend(12u32.to_be_bytes());
    out.extend(sub);
    out
}

/// A Windows `name` table naming the subset `family`, with the source's
/// copyright and license records.
fn name(source: &[u8], family: &str, style: &str) -> Vec<u8> {
    let (count, strings) = (u16_at(source, 2) as usize, u16_at(source, 4) as usize);
    let mut records: BTreeMap<u16, Vec<u8>> = (0..count)
        .map(|i| 6 + i * 12)
        .filter(|&r| {
            u16_at(source, r) == 3 && u16_at(source, r + 2) == 1 && u16_at(source, r + 4) == 0x409
        })
        .filter(|&r| COPIED_NAMES.contains(&u16_at(source, r + 6)))
        .map(|r| {
            let (len, off) = (
                u16_at(source, r + 8) as usize,
                u16_at(source, r + 10) as usize,
            );
            (
                u16_at(source, r + 6),
                source[strings + off..strings + off + len].to_vec(),
            )
        })
        .collect();
    let utf16 = |s: &str| {
        s.encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<u8>>()
    };
    let postscript = format!("{}-{style}", family.replace(' ', ""));
    for (id, text) in [
        (1, family.to_string()),
        (2, style.to_string()),
        (3, postscript.clone()),
        (4, format!("{family} {style}")),
        (6, postscript),
    ] {
        records.insert(id, utf16(&text));
    }

    let mut out = Vec::new();
    for v in [0, records.len() as u16, 6 + 12 * records.len() as u16] {
        out.extend(v.to_be_bytes());
    }
    let mut data: Vec<u8> = Vec::new();
    for (id, text) in &records {
        for v in [3, 1, 0x409, *id, text.len() as u16, data.len() as u16] {
            out.extend(v.to_be_bytes());
        }
        data.extend(text);
    }
    out.extend(data);
    out
}

/// Assembles tables into a TrueType file and sets `head.checkSumAdjustment`.
fn sfnt(mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    tables.sort_by_key(|(tag, _)| *tag);
    let n = tables.len() as u16;
    let mut out = Vec::new();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend(n.to_be_bytes());
    out.extend(search_fields(n, 16));
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in &tables {
        out.extend(tag);
        out.extend(checksum(data).to_be_bytes());
        out.extend((offset as u32).to_be_bytes());
        out.extend((data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    let mut head_at = 0;
    for (tag, data) in &tables {
        if tag == b"head" {
            head_at = out.len();
        }
        out.extend(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    let adjustment = 0xb1b0_afbau32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adjustment.to_be_bytes());
    out
}

/// `searchRange`, `entrySelector` and `rangeShift` for `n` entries of `size` bytes.
fn search_fields(n: u16, size: u16) -> Vec<u8> {
    let log = 15 - n.leading_zeros() as u16;
    let range = size << log;
    [range, log, n * size - range]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect()
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, c| {
        let mut word = [0; 4];
        word[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn u16_at(d: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([d[i], d[i + 1]])
}

fn u32_at(d: &[u8], i: usize) -> u32 {
    u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::fonts::{BOLD, MONO, SANS};

    fn chars(s: &str) -> BTreeSet<char> {
        s.chars().collect()
    }

    #[test]
    fn keeps_advances_outlines_and_mapping() {
        let text = "Hello, layup! é·→ {}\u{a0}";
        for data in [SANS, BOLD, MONO] {
            let original = Face::parse(data, 0).unwrap();
            let (out, _) = subset(data, &chars(text), "Layup Sans", "Regular");
            let face = Face::parse(&out, 0).expect("subset parses");
            assert!(face.number_of_glyphs() < 40);
            assert_eq!(face.units_per_em(), original.units_per_em());
            for c in text.chars() {
                let (Some(a), Some(b)) = (original.glyph_index(c), face.glyph_index(c)) else {
                    assert!(original.glyph_index(c).is_none(), "{c:?} lost");
                    continue;
                };
                assert_eq!(
                    original.glyph_hor_advance(a),
                    face.glyph_hor_advance(b),
                    "{c:?}"
                );
                assert_eq!(
                    original.glyph_bounding_box(a),
                    face.glyph_bounding_box(b),
                    "{c:?}"
                );
            }
            assert!(face.glyph_index('Z').is_none());
            assert_eq!(checksum(&out), 0xb1b0_afba);
        }
    }

    #[test]
    fn renames_the_family_and_keeps_the_license() {
        let (out, _) = subset(SANS, &chars("a"), "Layup Sans", "SemiBold");
        let face = Face::parse(&out, 0).unwrap();
        let names: Vec<String> = face
            .names()
            .into_iter()
            .filter_map(|n| n.to_string())
            .collect();
        assert!(names.iter().any(|n| n == "Layup Sans"));
        assert!(names.iter().any(|n| n == "LayupSans-SemiBold"));
        assert!(names.iter().any(|n| n.contains("Open Font License")));
        assert!(
            !names
                .iter()
                .any(|n| n.contains("Plex") && !n.contains("Copyright"))
        );
    }

    #[test]
    fn keeps_composite_components() {
        // Accented letters in these fonts may be composites of base glyphs.
        let (out, _) = subset(SANS, &chars("éÅ"), "Layup Sans", "Regular");
        let face = Face::parse(&out, 0).unwrap();
        for g in 0..face.number_of_glyphs() {
            let data = RawFace::parse(&out, 0).unwrap();
            let glyf = data.table(Tag::from_bytes(b"glyf")).unwrap();
            let loca = data.table(Tag::from_bytes(b"loca")).unwrap();
            let (a, b) = (
                u32_at(loca, g as usize * 4) as usize,
                u32_at(loca, g as usize * 4 + 4) as usize,
            );
            for (_, c) in components(&glyf[a..b]) {
                assert!(c < face.number_of_glyphs());
            }
        }
    }
}
