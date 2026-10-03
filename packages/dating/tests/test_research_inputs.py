"""Proposed regressions: dependencies through analysis inputs are still dependencies."""
import unittest
from harness.research import validate_graph


def graph(relation, kind, reverse=True):
    edges = [dict(source='a', target='input', relation=relation,
                  basis='Synthetic dependency fixture', references=['fixture'])]
    if reverse:
        edges.append(dict(source='input', target='a', relation='derived_from',
                          basis='Synthetic dependency fixture', references=['fixture']))
    return dict(nodes=[dict(id='a', kind='analysis', label='analysis'),
                       dict(id='input', kind=kind, label='input')], relations=edges)


class InputDependencyRegressions(unittest.TestCase):
    def test_dataset_feedback_is_cycle(self):
        self.assertIn('research dependency cycle', validate_graph(graph('uses_dataset', 'dataset')))

    def test_calibration_feedback_is_cycle(self):
        self.assertIn('research dependency cycle', validate_graph(graph('uses_calibration_set', 'calibration_set')))

    def test_acyclic_inputs_are_valid(self):
        for relation, kind in [('uses_dataset', 'dataset'), ('uses_calibration_set', 'calibration_set')]:
            self.assertEqual(validate_graph(graph(relation, kind, False)), [])

    def test_shared_inputs_are_valid(self):
        g = graph('uses_dataset', 'dataset', False)
        g['nodes'].append(dict(id='b', kind='analysis', label='second analysis'))
        g['relations'].append(dict(source='b', target='input', relation='uses_dataset',
                                  basis='Shared data are allowed', references=['fixture']))
        self.assertEqual(validate_graph(g), [])

    def test_scholarly_reply_is_not_derivation(self):
        g = dict(nodes=[dict(id=x, kind='study', label=x) for x in ['s1', 's2']],
                 relations=[dict(source='s1', target='s2', relation='critiques', basis='Discussion', references=['fixture']),
                            dict(source='s2', target='s1', relation='replies_to', basis='Discussion', references=['fixture'])])
        self.assertEqual(validate_graph(g), [])


if __name__ == '__main__':
    unittest.main()
