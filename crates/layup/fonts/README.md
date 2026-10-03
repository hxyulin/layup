# Bundled fonts

Source: https://github.com/IBM/plex/tree/78cd4223d8de9fcb78cba84eadecb269c56093c5
Unmodified IBM Plex fonts, licensed under SIL OFL 1.1 (see OFL.txt). Rendered
output embeds subsets of these fonts renamed Layup Sans and Layup Mono, since
the OFL reserves the name "Plex" for unmodified versions; each SVG carries the
copyright and license notice.

SHA-256:

- `IBMPlexSans-Regular.ttf`: `975dcda37d80f038dcd143c22e33ca2d97a0cc5a929aace1c749153b0fe1afa5`
- `IBMPlexSans-SemiBold.ttf`: `a20caf8286023a6a7a85e40b1d2a4ae9fc3e3b1f9eda8f4c542dd4986af67bb1`
- `IBMPlexMono-Regular.ttf`: `7c6fbddca4b700be918f5f6183d9bd4464fa427fe435f0b480d77fe2bb8c5a43`
- `OFL.txt`: `91c25c350d3cac39da2736d74f7ba37ef648f5237a4e330a240615bc8d8c4360`

Arabic and Hebrew regular/semibold faces are unmodified files from
[IBM Plex revision 763c36ef9117782905ae010056dfbe8fd2653a25](https://github.com/IBM/plex/tree/763c36ef9117782905ae010056dfbe8fd2653a25),
under `packages/plex-sans-{arabic,hebrew}/fonts/complete/ttf/`. They use the
same SIL OFL and are embedded intact only when needed, preserving GSUB/GPOS
shaping tables. Their CSS aliases are Layup Arabic and Layup Hebrew.

SHA-256:

- `IBMPlexSansArabic-Regular.ttf`: `8e0f1046c736bf939d4939ee3ae0116acf61cbcd6592deae7656761627080981`
- `IBMPlexSansArabic-SemiBold.ttf`: `1d4ab8b6ebadbb9d85f2ecdcb7cb0bb672420a8ff3388ab4f69d7b98056562aa`
- `IBMPlexSansHebrew-Regular.ttf`: `98cd8ca13fef47fb57c20faed17a346639bb418b7abeb096fd9693ea3eecc445`
- `IBMPlexSansHebrew-SemiBold.ttf`: `7d90b813182ef463b1e7cf8515699e94946003c6e3b2d0ceebc1825c4fc5b955`

No CJK font is bundled. CJK uses system fallback and width estimates unless
font bytes are explicitly supplied with the CLI, Rust, or JavaScript API.
