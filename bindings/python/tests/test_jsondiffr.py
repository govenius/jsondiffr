"""Python bindings tests.

Run with the jsondiff1 conda env active and after `maturin develop` has
installed the extension.
"""
import json
import pathlib

import pytest

import jsondiffr


REF_DIR = pathlib.Path(__file__).resolve().parents[3] / "reference_data"


def _cases():
    manifest = json.loads((REF_DIR / "_manifest.json").read_text())
    return manifest["cases"]


@pytest.mark.parametrize("case_id", _cases())
def test_reference_fixture(case_id):
    fixture = json.loads((REF_DIR / f"{case_id}.json").read_text())
    source_str = json.dumps(fixture["source"])
    diff_str = json.dumps(fixture["diff"])
    result = jsondiffr.patch(source_str, diff_str)
    assert result.to_python() == fixture["target"]


def test_jsonvalue_amortized_source():
    src = jsondiffr.JsonValue.from_json('{"a": 1}')
    out1 = jsondiffr.patch(src, '{"b": 2}')
    out2 = jsondiffr.patch(src, '{"c": 3}')
    assert out1.to_python() == {"a": 1, "b": 2}
    assert out2.to_python() == {"a": 1, "c": 3}


def test_patch_rejects_bad_source_type():
    with pytest.raises(TypeError):
        jsondiffr.patch(42, "{}")


def test_parse_error_maps_to_python():
    with pytest.raises(jsondiffr.ParseError):
        jsondiffr.JsonValue.from_json("not json")


def test_to_python_descends_into_object():
    jv = jsondiffr.JsonValue.from_json('{"a": {"b": [10, 20, 30]}}')
    assert jv.to_python(("a", "b")) == [10, 20, 30]
    assert jv.to_python(("a", "b", 1)) == 20


def test_to_python_empty_path_returns_root():
    jv = jsondiffr.JsonValue.from_json('{"a": 1}')
    assert jv.to_python() == {"a": 1}
    assert jv.to_python(()) == {"a": 1}


def test_to_python_missing_key_raises_key_error():
    jv = jsondiffr.JsonValue.from_json('{"a": 1}')
    with pytest.raises(KeyError):
        jv.to_python(("nope",))


def test_to_python_out_of_range_raises_index_error():
    jv = jsondiffr.JsonValue.from_json("[1, 2]")
    with pytest.raises(IndexError):
        jv.to_python((5,))


def test_to_python_type_mismatch_raises_type_error():
    jv = jsondiffr.JsonValue.from_json("[1, 2]")
    with pytest.raises(TypeError):
        jv.to_python(("a",))  # str into array
    jv = jsondiffr.JsonValue.from_json('{"a": 1}')
    with pytest.raises(TypeError):
        jv.to_python((0,))  # int into object


def test_jsonvalue_equality():
    a = jsondiffr.JsonValue.from_json('{"x": 1}')
    b = jsondiffr.JsonValue.from_json('{"x": 1}')
    c = jsondiffr.JsonValue.from_json('{"x": 2}')
    assert a == b
    assert a != c
