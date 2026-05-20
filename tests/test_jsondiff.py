import copy
import io
import json
import logging
import os.path
import sys
import unittest
import pytest

import jsondiff
from jsondiff import diff, replace, add, discard, insert, delete, JsonDiffer

from .utils import generate_random_json, perturbate_json

from hypothesis import given, settings, strategies


logging.basicConfig(
    level=logging.INFO,
    format=(
        '%(asctime)s %(pathname)s[line:%(lineno)d] %(levelname)s %(message)s'
    ),
)
logging.getLogger("jsondiff").setLevel(logging.DEBUG)


def generate_scenario(rng):
    a = generate_random_json(rng, sets=True)
    b = perturbate_json(a, rng, sets=True)
    return a, b


def generate_scenario_no_sets(rng):
    a = generate_random_json(rng, sets=False)
    b = perturbate_json(a, rng, sets=False)
    return a, b


class JsonDiffTests(unittest.TestCase):

    def test_a(self):

        self.assertEqual({}, diff(1, 1))
        self.assertEqual({}, diff(True, True))
        self.assertEqual({}, diff('abc', 'abc'))
        self.assertEqual({}, diff([1, 2], [1, 2]))
        self.assertEqual({}, diff((1, 2), (1, 2)))
        self.assertEqual({}, diff({1, 2}, {1, 2}))
        self.assertEqual({}, diff({'a': 1, 'b': 2}, {'a': 1, 'b': 2}))
        self.assertEqual({}, diff([], []))
        self.assertEqual({}, diff(None, None))
        self.assertEqual({}, diff({}, {}))
        self.assertEqual({}, diff(set(), set()))

        self.assertEqual(2, diff(1, 2))
        self.assertEqual(False, diff(True, False))
        self.assertEqual('def', diff('abc', 'def'))
        self.assertEqual([3, 4], diff([1, 2], [3, 4]))
        self.assertEqual((3, 4), diff((1, 2), (3, 4)))
        self.assertEqual({3, 4}, diff({1, 2}, {3, 4}))
        self.assertEqual({replace: {'c': 3, 'd': 4}}, diff({'a': 1, 'b': 2}, {'c': 3, 'd': 4}))

        self.assertEqual({replace: {'c': 3, 'd': 4}}, diff([1, 2], {'c': 3, 'd': 4}))
        self.assertEqual(123, diff({'a': 1, 'b': 2}, 123))

        self.assertEqual({delete: ['b']}, diff({'a': 1, 'b': 2}, {'a': 1}))
        self.assertEqual({'b': 3}, diff({'a': 1, 'b': 2}, {'a': 1, 'b': 3}))
        self.assertEqual({'c': 3}, diff({'a': 1, 'b': 2}, {'a': 1, 'b': 2, 'c': 3}))
        self.assertEqual({delete: ['b'], 'c': 3}, diff({'a': 1, 'b': 2}, {'a': 1, 'c': 3}))

        self.assertEqual({add: {3}}, diff({1, 2}, {1, 2, 3}))
        self.assertEqual({add: {3}, discard: {4}}, diff({1, 2, 4}, {1, 2, 3}))
        self.assertEqual({discard: {4}}, diff({1, 2, 4}, {1, 2}))

        self.assertEqual({insert: [(1, 'b')]}, diff(['a', 'c'], ['a', 'b', 'c']))
        self.assertEqual({insert: [(1, 'b')], delete: [3, 0]}, diff(['x', 'a', 'c', 'x'], ['a', 'b', 'c']))
        self.assertEqual(
            {insert: [(2, 'b')], delete: [4, 0], 1: {'v': 20}},
            diff(['x', 'a', {'v': 11}, 'c', 'x'], ['a', {'v': 20}, 'b', 'c'])
        )
        self.assertEqual(
            {insert: [(2, 'b')], delete: [4, 0],  1: {'v': 20}},
            diff(['x', 'a', {'u': 10, 'v': 11}, 'c', 'x'], ['a', {'u': 10, 'v': 20}, 'b', 'c'])
        )

    def test_marshal(self):
        differ = JsonDiffer()

        d = {
            delete: 3,
            '$delete': 4,
            insert: 4,
            '$$something': 1
        }

        dm = differ.marshal(d)

        self.assertEqual(d, differ.unmarshal(dm))

    @given(strategies.randoms().map(generate_scenario_no_sets))
    @settings(max_examples=1000)
    def test_dump(self, scenario):
        a, b = scenario
        diff(a, b, dump=True)

    @given(strategies.randoms().map(generate_scenario))
    @settings(max_examples=1000)
    def test_compact_syntax(self, scenario):
        a, b = scenario
        differ = JsonDiffer()
        d = differ.diff(a, b)
        self.assertEqual(b, differ.patch(a, d))
        dm = differ.marshal(d)
        self.assertEqual(d, differ.unmarshal(dm))

    def test_long_arrays(self):
        size = 100
        a = [{'a': i, 'b': 2 * i} for i in range(1, size)]
        b = [{'a': i, 'b': 3 * i} for i in range(1, size)]
        r = sys.getrecursionlimit()
        sys.setrecursionlimit(size - 1)

        try:
            diff(a, b)
        except RecursionError:
            self.fail('cannot diff long arrays')
        finally:
            sys.setrecursionlimit(r)


@pytest.mark.parametrize(
    ("a", "b", "expected"),
    [
        pytest.param([], [{"a": True}], [{"a": True}],
                     id="issue59_"),
        pytest.param([{"a": True}], [], [],
                     id="issue59_"),
        pytest.param({1: 2}, {5: 3}, {replace: {5: 3}},
                     id="issue36_"),
    ],
)
class TestSpecificIssue:
    def test_issue(self, a, b, expected):
        actual = diff(a, b)
        assert actual == expected


class TestFormatConformance(unittest.TestCase):
    """Tests that verify conformance to FORMAT.md."""

    # ----- §1 Marshalled format -----

    def test_marshal_escape_chain(self):
        differ = JsonDiffer()
        self.assertEqual({'$$x': 1}, differ.marshal({'$x': 1}))
        self.assertEqual({'$$$x': 1}, differ.marshal({'$$x': 1}))
        self.assertEqual({'$x': 1}, differ.unmarshal({'$$x': 1}))

    def test_marshal_does_not_escape_non_string_keys(self):
        d = {1: {'$v': 2}, insert: [(0, 'x')]}
        m = JsonDiffer().marshal(d)
        self.assertIn(1, m)
        self.assertIn('$insert', m)
        self.assertEqual({'$$v': 2}, m[1])

    def test_json_wire_fidelity(self):
        a = {'k': [1, 2, 3]}
        b = {'k': [1, 3, 4]}
        m = diff(a, b, marshal=True)
        roundtripped = json.loads(json.dumps(m))
        differ = JsonDiffer()
        self.assertEqual(b, differ.patch(a, differ.unmarshal(roundtripped)))

    # ----- §2 patch() behavior -----

    def test_patch_empty_diff_is_identity(self):
        for a in [{}, [], (1, 2), set(), {'a': 1}, [1, 2, 3], 0, 'x', None]:
            self.assertEqual(a, JsonDiffer().patch(a, {}))

    def test_patch_non_dict_diff_is_replacement(self):
        self.assertEqual([1, 2], JsonDiffer().patch({'a': 1}, [1, 2]))
        self.assertEqual(42, JsonDiffer().patch({'a': 1}, 42))
        self.assertEqual('new', JsonDiffer().patch([1, 2], 'new'))

    def test_patch_replace_sentinel(self):
        self.assertEqual({'x': 9}, JsonDiffer().patch({'a': 1}, {replace: {'x': 9}}))
        self.assertEqual('gone', JsonDiffer().patch([1, 2], {replace: 'gone'}))

    def test_patch_dict_delete_then_add(self):
        result = JsonDiffer().patch({'a': 1, 'b': 2}, {'c': 3, delete: ['a']})
        self.assertEqual({'b': 2, 'c': 3}, result)

    def test_patch_list_delete_descending_order(self):
        self.assertEqual(['a', 'c'], JsonDiffer().patch(['a', 'b', 'c', 'd'], {delete: [3, 1]}))

    def test_patch_list_integer_keyed_subpatch(self):
        src = [{'x': 1}, 'b']
        self.assertEqual([{'x': 2}, 'b'], JsonDiffer().patch(src, {0: {'x': 2}}))

    def test_patch_preserves_tuple_type(self):
        result = JsonDiffer().patch((1, 2, 3), {0: 9})
        self.assertEqual((9, 2, 3), result)
        self.assertIsInstance(result, tuple)
        self.assertIsInstance(JsonDiffer().patch((1, 2), {}), tuple)

    def test_patch_set_add_and_discard(self):
        self.assertEqual({1, 3, 9}, JsonDiffer().patch({1, 2, 3}, {discard: {2}, add: {9}}))
        self.assertEqual({1, 2, 9}, JsonDiffer().patch({1, 2}, {add: {9}}))
        self.assertEqual({1, 3}, JsonDiffer().patch({1, 2, 3}, {discard: {2}}))

    # ----- §3 diff() behavior -----

    def test_diff_low_similarity_emits_bare_or_wrapped_b(self):
        self.assertEqual(['x', 'y'], diff([1, 2, 3], ['x', 'y']))
        self.assertEqual({replace: {'b': 2}}, diff({'a': 1}, {'b': 2}))
        self.assertEqual({replace: {'a': 1}}, diff([1, 2], {'a': 1}))
        self.assertEqual([1, 2], diff({'a': 1}, [1, 2]))

    def test_diff_list_delete_positions_descending(self):
        d = diff(['x', 'a', 'c', 'x'], ['a', 'c'])
        positions = d[delete]
        self.assertEqual(sorted(positions, reverse=True), positions)

    def test_diff_unchanged_subkeys_omitted(self):
        d = diff({'a': 1, 'b': 2}, {'a': 1, 'b': 3})
        self.assertNotIn('a', d)
        self.assertEqual(3, d.get('b'))

    def test_diff_against_empty_uses_replace_wrap(self):
        self.assertEqual({replace: {'a': 1}}, diff({}, {'a': 1}))
        self.assertEqual({replace: {}}, diff({'a': 1}, {}))

    # ----- §4 Invariants -----

    def test_diff_identity(self):
        for a in [{}, [], (1, 2), set(), {'a': 1}, [1, 2, 3], 0, 'x', None, True]:
            self.assertEqual({}, diff(a, a))

    def test_no_unpatch_method(self):
        self.assertFalse(hasattr(JsonDiffer, 'unpatch'))

    # ----- Cross-cutting -----

    def test_diff_does_not_mutate_inputs(self):
        a = {'k': [1, 2, 3], 'nested': {'x': 1}}
        b = {'k': [1, 3, 4], 'nested': {'x': 2}}
        a_orig = copy.deepcopy(a)
        b_orig = copy.deepcopy(b)
        d = diff(a, b)
        d_orig = copy.deepcopy(d)
        JsonDiffer().patch(a, d)
        self.assertEqual(a_orig, a)
        self.assertEqual(b_orig, b)
        self.assertEqual(d_orig, d)

    def test_string_delete_is_not_a_sentinel(self):
        d = diff({'delete': 1}, {'delete': 2})
        self.assertEqual({'delete': 2}, d)
        self.assertNotIn(delete, d)


class TestLoaders(unittest.TestCase):

    here = os.path.dirname(__file__)
    data_dir = os.path.join(here, "data")

    def test_json_string_loader(self):
        json_str = '{"hello": "world", "data": [1,2,3]}'
        expected = {"hello": "world", "data": [1, 2, 3]}
        loader = jsondiff.JsonLoader()
        actual = loader(json_str)
        self.assertEqual(expected, actual)

    def test_json_file_loader(self):
        json_file = os.path.join(TestLoaders.data_dir, "test_01.json")
        expected = {"hello": "world", "data": [1, 2, 3]}
        loader = jsondiff.JsonLoader()
        with open(json_file) as f:
            actual = loader(f)
        self.assertEqual(expected, actual)

    def test_yaml_string_loader(self):
        json_str = """---
hello: world
data:
  - 1
  - 2
  - 3
        """
        expected = {"hello": "world", "data": [1, 2, 3]}
        loader = jsondiff.YamlLoader()
        actual = loader(json_str)
        self.assertEqual(expected, actual)

    def test_yaml_file_loader(self):
        yaml_file = os.path.join(TestLoaders.data_dir, "test_01.yaml")
        expected = {"hello": "world", "data": [1, 2, 3]}
        loader = jsondiff.YamlLoader()
        with open(yaml_file) as f:
            actual = loader(f)
        self.assertEqual(expected, actual)


class TestDumpers(unittest.TestCase):

    def test_json_dump_string(self):
        data = {"hello": "world", "data": [1, 2, 3]}
        expected = '{"hello": "world", "data": [1, 2, 3]}'
        dumper = jsondiff.JsonDumper()
        actual = dumper(data)
        self.assertEqual(expected, actual)

    def test_json_dump_string_indented(self):
        data = {"hello": "world", "data": [1, 2, 3]}
        expected = """{
    "hello": "world",
    "data": [
        1,
        2,
        3
    ]
}"""
        dumper = jsondiff.JsonDumper(indent=4)
        actual = dumper(data)
        self.assertEqual(expected, actual)

    def test_json_dump_string_fp(self):
        data = {"hello": "world", "data": [1, 2, 3]}
        expected = """{
    "hello": "world",
    "data": [
        1,
        2,
        3
    ]
}"""
        dumper = jsondiff.JsonDumper(indent=4)
        buffer = io.StringIO()
        dumper(data, buffer)
        self.assertEqual(expected, buffer.getvalue())

    def test_yaml_dump_string(self):
        data = {"hello": "world", "data": [1, 2, 3]}
        expected = """data:
- 1
- 2
- 3
hello: world
"""
        # Sort keys just to make things deterministic
        dumper = jsondiff.YamlDumper(sort_keys=True)
        actual = dumper(data)
        self.assertEqual(expected, actual)

    def test_yaml_dump_string_fp(self):
        data = {"hello": "world", "data": [1, 2, 3]}
        expected = """data:
- 1
- 2
- 3
hello: world
"""
        # Sort keys just to make things deterministic
        dumper = jsondiff.YamlDumper(sort_keys=True)
        buffer = io.StringIO()
        dumper(data, buffer)
        self.assertEqual(expected, buffer.getvalue())

    def test_exclude_paths(self):
        differ = JsonDiffer()

        a = {'a': 1, 'b': {'b1': 20, 'b2': 21}, 'c': 3}
        b = {'a': 1, 'b': {'b1': 22, 'b2': 23}, 'c': 30}

        exclude_paths = ['b.b1', 'c']

        d = differ.diff(a, b, exclude_paths=exclude_paths)

        # The diff should only contain changes that are not in the exclude_paths
        self.assertEqual({'b': {'b2': 23}}, d)
