import timeit
import copy

from jsondiff import patch, diff, JsonDiffer; unmarshal = JsonDiffer().unmarshal

def generate_large_dict(size):
    return {f"key_{i}": i for i in range(size)}

def generate_deep_dict(depth, width):
    if depth <= 0:
        return {"value": "end"}
    return {f"level_{depth}_{i}": generate_deep_dict(depth - 1, width) for i in range(width)}

def generate_large_list(size):
    return list(range(size))

def run_benchmark(name, a, d, iterations=20, do_patch=True, do_diff=True, jsondiffr=False):
    print(f"Benchmarking: {name}")

    if do_patch:
        setup_code = f"import json; import jsondiff; a={a}; d=json.dumps({d})"
        stmt = f"jsondiff.patch(a, json.loads(d), marshal=True)" # Typical use case: a = Python dict, d = (marshalled) str, output = Python dict
        if jsondiffr:
            setup_code = f"import json; from jsondiffr import patch; a=patch(json.dumps({a}), '" "{}" f"'); d=json.dumps({d})"
            stmt = f"patch(a, d)" # Typical use case: a = JsonValue (previously patched JSON), d = (marshalled) str, output = JsonValue (assuming only small subdicts queried/converted to Python later)
        try:
            timer = timeit.Timer(stmt, setup=setup_code)
            result = timer.timeit(number=iterations)
            print(f"  patch(a,d): {result/iterations:.6f} seconds per iteration ({iterations} iterations){ " - jsondiffr" if jsondiffr else ""}")
        except Exception as e:
            print(f"  FAILED: {e}")

    if do_diff:
        b = patch(copy.deepcopy(a), unmarshal(copy.deepcopy(d)))
        setup_code = f"import json; from jsondiff import diff; a={a}; b={b}"
        stmt = "json.dumps(diff(a, b, marshal=True))" # Typical use case: a,b = Python dict, output = (marshalled) str
        if jsondiffr:
            setup_code = f"import json; from jsondiffr import diff; a=json.dumps({a}); b=json.dumps({b})"
            stmt = "diff(a, b)" # Typical use case: a,b = Python dict, output = (marshalled) str
        try:
            timer = timeit.Timer(stmt, setup=setup_code)
            result = timer.timeit(number=iterations)
            print(f"   diff(a,b): {result/iterations:.6f} seconds per iteration ({iterations} iterations){ " - jsondiffr" if jsondiffr else ""}")
        except Exception as e:
            print(f"  FAILED: {e}")

    print("")

if __name__ == "__main__":
    print("Starting jsondiff benchmarks...\n")

    # 1. Large Flat Dict
    n_elements = 50000; n_mods = n_elements // 10
    a_flat = generate_large_dict(n_elements)
    d_flat = {i: i*2 for i in range(0, n_elements, 10)}

    for r in [True, False]: run_benchmark(f"Large Flat Dict ({n_elements} keys, {n_mods} updates)", a_flat, d_flat, jsondiffr=r)

    # 2. Deeply Nested Dict
    a_deep = generate_deep_dict(5, 6)
    d_deep = {"level_5_0": {"level_4_0": {"level_3_0": {"level_2_0": {"level_1_0": {"value": "new_value"}}}}}}

    for r in [True, False]: run_benchmark(f"Deeply Nested Dict (Depth 5)", a_deep, d_deep, jsondiffr=r)

    # 3. Large List (Identity/No change)
    n_elements = 10000
    a_list = generate_large_list(n_elements)
    d_empty = {}
    for r in [True, False]: run_benchmark(f"Large List ({n_elements} elements, empty diff)", a_list, d_empty, jsondiffr=r, do_diff=r)

    # 4. Large List (Many insertions/deletions)
    n_elements = 1000; n_mods = n_elements // 10
    a_list_mod = generate_large_list(n_elements)
    d_list_mod = {"$delete": list(reversed(range(n_mods))), "$insert": [(n_elements//2, i) for i in range(n_mods)]}
    for r in [True, False]: run_benchmark(f"Large List ({n_elements} elements, {n_mods} del + {n_mods} ins)", a_list_mod, d_list_mod, jsondiffr=r)

    # 5. Large List (Element updates)
    n_elements = 2000; n_mods = n_elements // 5
    a_list_upd = generate_large_list(n_elements)
    d_list_upd = {i: i + 1 for i in range(0, n_elements, n_elements//n_mods)}

    for r in [True, False]: run_benchmark(f"Large List ({n_elements} elements, {n_mods} updates)", a_list_upd, d_list_upd, jsondiffr=r)

    print("Benchmarks complete.")
