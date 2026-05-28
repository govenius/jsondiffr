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

if __name__ == "__main__":
    print("Starting jsondiff.patch benchmarks...\n")

    # --- Scenario 1: Large Flat Dicts ---
    size = 1000
    a_dict = generate_large_dict(size)
    # Diff: change every 10th element
    b_dict = generate_large_dict(size)
    diff_dict = {i: i + 1 for i in range(0, size, 10)}

    # Let's use a cleaner approach for the actual execution block
    
    # 1. Large Flat Dict
    a_flat = generate_large_dict(50000)
    d_flat = {i: i*2 for i in range(0, 50000, 10)}
    run_benchmark("Large Flat Dict (50000 keys, 5000 updates)", a_flat, d_flat, 100)

    # 2. Deeply Nested Dict
    # We'll patch one value at the very bottom
    a_deep = generate_deep_dict(5, 6)
    # Path to a leaf: level_4_0 -> level_3_0 -> level_2_0 -> level_1_0
    d_deep = {"level_5_0": {"level_4_0": {"level_3_0": {"level_2_0": {"level_1_0": {"value": "new_value"}}}}}}
    run_benchmark("Deeply Nested Dict (Depth 5)", a_deep, d_deep, 100)

    # 3. Large List (Identity/No change)
    a_list = generate_large_list(50000)
    d_empty = {}
    run_benchmark("Large List (50000 elements, empty diff)", a_list, d_empty, 500)

    # 4. Large List (Many insertions/deletions)
    a_list_mod = generate_large_list(10000)
    # Delete first 100, insert 100 at the end
    d_list_mod = {delete: list(range(1000)), insert: [(10000, i) for i in range(1000)]}
    run_benchmark("Large List (10000 elements, 1000 del + 1000 ins)", a_list_mod, d_list_mod, 1000)

    # 5. Large List (Element updates)
    a_list_upd = generate_large_list(20000)
    # Update every 5th element
    d_list_upd = {i: i + 1 for i in range(0, 20000, 5)}
    run_benchmark("Large List (20000 elements, 4000 updates)", a_list_upd, d_list_upd, 100)

    print("Benchmarks complete.")
