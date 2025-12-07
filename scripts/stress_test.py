import requests
import time
import json
import random

LEADER_URL = "http://localhost:3001/submit"

# LeetCode #11: Container With Most Water
# Given n non-negative integers a1, a2, ..., an, where each represents a point at coordinate (i, ai).
# n vertical lines are drawn such that the two endpoints of the line i is at (i, ai) and (i, 0).
# Find two lines, which, together with the x-axis forms a container, such that the container contains the most water.
JAVA_CODE = """
import java.util.Scanner;

class Solution {
    public int maxArea(int[] height) {
        int left = 0;
        int right = height.length - 1;
        int maxArea = 0;
        
        while (left < right) {
            int width = right - left;
            int h = Math.min(height[left], height[right]);
            int area = width * h;
            maxArea = Math.max(maxArea, area);
            
            if (height[left] < height[right]) {
                left++;
            } else {
                right--;
            }
        }
        return maxArea;
    }
}

public class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner(System.in);
        int n = sc.nextInt();
        int[] height = new int[n];
        for (int i = 0; i < n; i++) {
            height[i] = sc.nextInt();
        }
        Solution sol = new Solution();
        System.out.println(sol.maxArea(height));
    }
}
"""

def compute_max_area(heights):
    """Python implementation to generate expected output."""
    left, right = 0, len(heights) - 1
    max_area = 0
    while left < right:
        width = right - left
        h = min(heights[left], heights[right])
        max_area = max(max_area, width * h)
        if heights[left] < heights[right]:
            left += 1
        else:
            right -= 1
    return max_area

def generate_testcases(n):
    """Generate n random testcases for Container With Most Water."""
    testcases = []
    for i in range(n):
        # Generate array of random heights (2 to 100 elements, heights 1-10000)
        size = random.randint(2, 100)
        heights = [random.randint(1, 10000) for _ in range(size)]
        
        # Format input: first line is size, second line is space-separated heights
        input_str = f"{size}\n" + " ".join(map(str, heights))
        expected_output = str(compute_max_area(heights))
        
        testcases.append({
            "id": i,
            "input": input_str,
            "output": expected_output
        })
    return testcases

def run_test(num_testcases=100):
    testcases = generate_testcases(num_testcases)
    
    payload = {
        "user_id": "leetcode_stress_tester",
        "language": "java",
        "code": JAVA_CODE,
        "testcases": testcases
    }

    print(f"=== Container With Most Water Stress Test ===")
    print(f"Problem: LeetCode #11 (Medium)")
    print(f"Testcases: {num_testcases}")
    print(f"Array sizes: 2-100 elements, Heights: 1-10000")
    print()
    
    print("Sending request to leader...")
    start_time = time.time()
    try:
        response = requests.post(LEADER_URL, json=payload)
        end_time = time.time()
        
        print(f"Status Code: {response.status_code}")
        
        if response.status_code == 200:
            data = response.json()
            print(f"\n=== Results ===")
            print(f"Overall Passed: {data['passed']}")
            print(f"Total Results: {len(data['results'])}")
            print(f"Time Taken: {end_time - start_time:.2f}s")
            
            # Group results by worker
            worker_stats = {}
            for res in data['results']:
                worker = res.get('worker_id', 'Unknown')
                if worker not in worker_stats:
                    worker_stats[worker] = {'passed': 0, 'failed': 0}
                if res['passed']:
                    worker_stats[worker]['passed'] += 1
                else:
                    worker_stats[worker]['failed'] += 1
            
            print(f"\n=== Worker Distribution ===")
            for worker, stats in worker_stats.items():
                total = stats['passed'] + stats['failed']
                print(f"Worker {worker[:8]}...: {stats['passed']}/{total} passed")
            
            # Show random samples
            print(f"\n=== Random Samples (5 of {len(data['results'])}) ===")
            sample_indices = random.sample(range(len(data['results'])), min(5, len(data['results'])))
            for idx in sorted(sample_indices):
                res = data['results'][idx]
                tc = testcases[res['id']]
                status = "✅" if res['passed'] else "❌"
                # Parse input to show array size
                input_lines = tc['input'].split('\n')
                array_size = input_lines[0]
                heights_preview = input_lines[1][:40] + "..." if len(input_lines[1]) > 40 else input_lines[1]
                print(f"{status} TC {res['id']:3d} | Worker: {res['worker_id'][:8]}... | Size: {array_size:3s} | Heights: [{heights_preview}]")
                print(f"         | Expected: {tc['output']:8s} | Actual: {res['actual_output']:8s}")
            
            # Show failures if any
            failures = [res for res in data['results'] if not res['passed']]
            if failures:
                print(f"\n=== Failures ({len(failures)}) ===")
                for res in failures[:5]:  # Show first 5 failures
                    tc = testcases[res['id']]
                    print(f"TC {res['id']}:")
                    print(f"  Input (first 50 chars): {tc['input'][:50]}...")
                    print(f"  Expected: {tc['output']}")
                    print(f"  Actual: {res['actual_output']}")
                    print(f"  Error: {res['error']}")
                if len(failures) > 5:
                    print(f"  ... and {len(failures) - 5} more failures")
            else:
                print(f"\n✅ All {num_testcases} testcases passed!")
            
            if data.get('error'):
                print(f"\nError: {data['error']}")
        else:
            print(f"Error: {response.text}")

    except Exception as e:
        print(f"Request failed: {e}")

if __name__ == "__main__":
    run_test(100)
