import hello from '../../../../examples/hello.layup?raw';
import architecture from '../../../../examples/service-layers.layup?raw';
import decisions from '../../../../examples/decision-tree.layup?raw';
import states from '../../../../examples/state-machine.layup?raw';
import composite from '../../../../examples/state-composite.layup?raw';
import sequence from '../../../../examples/sequence.layup?raw';
import international from '../../../../examples/sequence-international.layup?raw';
import rtl from '../../../../examples/right-to-left.layup?raw';
import slides from '../../../../examples/slides.layup?raw';
import presentation from '../../../../examples/presentation-model.layup?raw';
import models from '../../../../examples/model-views.layup?raw';

export const samples = [
  { id: 'hello', title: 'First diagram', source: hello },
  { id: 'architecture', title: 'Service architecture', source: architecture },
  { id: 'decisions', title: 'Support decision tree', source: decisions },
  { id: 'states', title: 'Job state machine', source: states },
  { id: 'composite', title: 'Composite connection states', source: composite },
  { id: 'sequence', title: 'Request sequence', source: sequence },
  { id: 'international', title: 'CJK and Arabic sequence', source: international },
  { id: 'rtl', title: 'Right-to-left graph', source: rtl },
  { id: 'slides', title: 'Slide-sized pipeline', source: slides },
  { id: 'presentation', title: 'Request/retry presentation', source: presentation, view: 'walkthrough' },
  { id: 'models', title: 'Shared architecture views', source: models, view: 'overview' },
];

export function sampleById(id) { return samples.find(sample => sample.id === id) || samples[0]; }
