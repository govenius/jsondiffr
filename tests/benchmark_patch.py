import timeit
import json
import jsondiff
from jsondiff import patch, diff, replace, insert, delete

def generate_large_dict(size):
    return {f"key_{i}": i for i in range(size)}

def generate_deep_dict(depth, width):
    if depth <= 0:
        return {"value": "end"}
    return {f"level_{depth}_{i}": generate_deep_dict(depth - 1, width) for i in range(width)}

def generate_large_list(size):
    return list(range(size))

def run_benchmark(name, a, d, iterations=100):
    print(f"Benchmarking: {name}")
    setup_code = f"from jsondiff import patch, insert, delete; a={a}; d={d}"
    stmt = "patch(a, d)"
    try:
        timer = timeit.Timer(stmt, setup=setup_code)
        result = timer.timeit(number=iterations)
        print(f"  Result: {result/iterations:.6f} seconds per iteration ({iterations} iterations)\n")
    except Exception as e:
        print(f"  Failed: {e}\n")

def run_benchmark_diff(name, a, b, iterations=100):
    print(f"Benchmarking: {name}")
    setup_code = f"from jsondiff import diff; a={a}; b={b}"
    stmt = "diff(a, b)"
    try:
        timer = timeit.Timer(stmt, setup=setup_code)
        result = timer.timeit(number=iterations)
        print(f"  Result: {result/iterations:.6f} seconds per iteration ({iterations} iterations)\n")
    except Exception as e:
        print(f"  Failed: {e}\n")

if __name__ == "__main__":
    print("Starting jsondiff benchmarks...\n")

    # --- Scenario 1: Large Flat Dicts ---
    size = 1000
    a_dict = generate_large_dict(size)
    # Diff: change every 10th element
    b_dict = generate_large_dict(size)
    diff_dict = {i: i + 1 for i in range(0, size, 10)}
    # Note: the diff_dict above is actually the target for patch, 
    # but for diff(a, b) we need the target b.
    # Let's construct b properly for the diff benchmark.
    b_dict_for_diff = a_dict.copy()
    for i in range(0, size, 10):
        b_dict_for_diff[f"key_{i}"] = i + 1

    # 1. Large Flat Dict
    a_flat = generate_large_dict(50000)
    d_flat = {i: i*2 for i in range(0, 50000, 10)}
    # For diff, we need the target b. 
    # Since d_flat is a dict of {index: value}, we need to map it back to keys.
    b_flat = a_flat.copy()
    for i in range(0, 50000, 10):
        b_flat[f"key_{i}"] = i*2
    
    run_benchmark("Large Flat Dict (50000 keys, 5000 updates) - PATCH", a_flat, d_flat, 100)
    run_benchmark_diff("Large Flat Dict (50000 keys, 5000 updates) - DIFF", a_flat, b_flat, 100)

    # 2. Deeply Nested Dict
    a_deep = generate_deep_dict(5, 6)
    d_deep = {"level_5_0": {"level_4_0": {"level_3_0": {"level_2_0": {"level_1_0": {"value": "new_value"}}}}}}
    # Construct b for diff
    import copy
    b_deep = copy.deepcopy(a_deep)
    # Manually traverse to set the value to match d_deep's structure
    curr = b_deep
    path = ["level_5_0", "level_4_0", "level_3_0", "level_2_0", "level_1_0"]
    for p in path[:-1]:
        curr = curr[p]
    curr[path[-1]]["value"] = "new_value"

    run_benchmark("Deeply Nested Dict (Depth 5) - PATCH", a_deep, d_deep, 100)
    run_benchmark_diff("Deeply Nested Dict (Depth 5) - DIFF", a_deep, b_deep, 100)

    # 3. Large List (Identity/No change)
    a_list = generate_large_list(50000)
    d_empty = {}
    run_benchmark("Large List (50000 elements, empty diff) - PATCH", a_list, d_empty, 500)
    run_benchmark_diff("Large List (50000 elements, empty diff) - DIFF", a_list, a_list, 500)

    # 4. Large List (Many insertions/deletions)
    a_list_mod = generate_large_list(10000)
    d_list_mod = {delete: list(range(1000)), insert: [(10000, i) for i in range(1000)]}
    # Construct b for diff
    b_list_mod = list(range(1000, 11000)) # This is a simplification, let's be precise
    # Original: 0...9999. Delete 0...999. Remaining: 1000...9999.
    # Insert 1000 elements at index 10000.
    # Result: [1000, 1001, ..., 9999, 0, 1, ..., 999] (if inserted at end)
    # Actually the benchmark uses (10000, i) which is at the end of the 10000 element list.
    b_list_mod = list(range(1000, 10000)) + list(range(1000))

    run_benchmark("Large List (10000 elements, 1000 del + 1000 ins) - PATCH", a_list_mod, d_list_mod, 1000)
    run_benchmark_diff("Large List (10000 elements, 1000 del + 1000 ins) - DIFF", a_list_mod, b_list_mod, 1000)

    # 5. Large List (Element updates)
    a_list_upd = generate_large_list(20000)
    d_list_upd = {i: i + 1 for i in range(0, 20000, 5)}
    # Construct b for diff
    b_list_upd = a_list_upd.copy()
    for i in range(0, 20000, 5):
        b_list_upd[i] = i + 1

    run_benchmark("Large List (20000 elements, 4000 updates) - PATCH", a_list_upd, d_list_upd, 100)
    run_benchmark_diff("Large List (20000 elements, 4000 updates) - DIFF", a_list_upd, b_list_upd, 100)

    print("Benchmarks complete.")
