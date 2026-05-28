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

def run_benchmark(name, setup_func, stmt, iterations=100):
    print(f"Benchmarking: {name}")
    try:
        timer = timeit.Timer(stmt, setup=setup_func)
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
    
    run_benchmark(
        "Large Flat Dict (1000 keys, 100 updates)",
        setup=lambda: (a_dict, diff_dict),
        stmt="patch(a, d)",
        iterations=500
    )
    # Note: The lambda in setup needs to provide the variables to the stmt scope.
    # Since timeit executes in a separate scope, we'll use a slightly different approach for the actual implementation.

    # Re-defining run_benchmark to handle scope better for this script
    def run_benchmark_fixed(name, a, d, iterations=100):
        print(f"Benchmarking: {name}")
        setup_code = f"from jsondiff import patch; a={a}; d={d}"
        stmt = "patch(a, d)"
        try:
            timer = timeit.Timer(stmt, setup=setup_code)
            result = timer.timeit(number=iterations)
            print(f"  Result: {result/iterations:.6f} seconds per iteration ({iterations} iterations)\n")
        except Exception as e:
            print(f"  Failed: {e}\n")

    # Let's use a cleaner approach for the actual execution block
    
    # 1. Large Flat Dict
    a_flat = generate_large_dict(5000)
    d_flat = {i: i*2 for i in range(0, 5000, 10)}
    run_benchmark_fixed("Large Flat Dict (5000 keys, 500 updates)", a_flat, d_flat, 100)

    # 2. Deeply Nested Dict
    # We'll patch one value at the very bottom
    a_deep = generate_deep_dict(4, 3) 
    # Find a path to a leaf
    # level_4_0 -> level_3_0 -> level_2_0 -> level_1_0
    d_deep = {"level_4_0": {"level_3_0": {"level_2_0": {"level_1_0": {"value": "new_value"}}}}}
    run_benchmark_fixed("Deeply Nested Dict (Depth 4)", a_deep, d_deep, 100)

    # 3. Large List (Identity/No change)
    a_list = generate_large_list(5000)
    d_empty = {}
    run_benchmark_fixed("Large List (5000 elements, empty diff)", a_list, d_empty, 500)

    # 4. Large List (Many insertions/deletions)
    a_list_mod = generate_large_list(1000)
    # Delete first 100, insert 100 at the end
    d_list_mod = {delete: list(range(100)), insert: [(1000, i) for i in range(100)]}
    run_benchmark_fixed("Large List (1000 elements, 100 del + 100 ins)", a_list_mod, d_list_mod, 100)

    # 5. Large List (Element updates)
    a_list_upd = generate_large_list(2000)
    # Update every 5th element
    d_list_upd = {i: i + 1 for i in range(0, 2000, 5)}
    run_benchmark_fixed("Large List (2000 elements, 400 updates)", a_list_upd, d_list_upd, 100)

    print("Benchmarks complete.")
