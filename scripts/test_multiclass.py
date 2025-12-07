import requests
import time
import json

LEADER_URL = "http://localhost:3001/submit"

JAVA_CODE = """
import java.util.Scanner;

class Helper {
    public int add(int a, int b) {
        return a + b;
    }
}

public class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner(System.in);
        if (sc.hasNextInt()) {
            int a = sc.nextInt();
            int b = sc.nextInt();
            Helper h = new Helper();
            System.out.println(h.add(a, b));
        }
    }
}
"""

def generate_testcases(n):
    testcases = []
    for i in range(n):
        testcases.append({
            "id": i,
            "input": f"{i} {i}",
            "output": f"{i + i}"
        })
    return testcases

def run_test(num_testcases=5):
    payload = {
        "user_id": "multiclass_tester",
        "language": "java",
        "code": JAVA_CODE,
        "testcases": generate_testcases(num_testcases)
    }

    print(f"Sending request with {num_testcases} testcases...")
    print("Request Payload:")
    print(json.dumps(payload, indent=2))
    
    start_time = time.time()
    try:
        response = requests.post(LEADER_URL, json=payload)
        end_time = time.time()
        
        print(f"Status Code: {response.status_code}")
        print("Response Payload:")
        try:
            data = response.json()
            print(json.dumps(data, indent=2))
            
            if response.status_code == 200:
                print(f"Passed: {data['passed']}")
                print(f"Total Results: {len(data['results'])}")
                print(f"Time Taken: {end_time - start_time:.2f}s")
                
                print("\nDetailed Results:")
                for res in data['results']:
                    status = "Passed" if res['passed'] else "Failed"
                    worker = res.get('worker_id', 'Unknown')
                    print(f"[Worker: {worker}] TC {res['id']}: {status} (Time: {res['time']})")
                    if not res['passed']:
                        print(f"  Expected: {res.get('expected_output', 'N/A')}") # Note: expected_output not in result yet, but good to have placeholder
                        print(f"  Actual: {res['actual_output']}")
                        print(f"  Error: {res['error']}")
        except json.JSONDecodeError:
            print("Response Text:", response.text)

    except Exception as e:
        print(f"Request failed: {e}")

if __name__ == "__main__":
    run_test(5)
