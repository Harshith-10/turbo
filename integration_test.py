import subprocess
import time
import requests
import os
import signal

def run_test():
    print("Building project...")
    subprocess.check_call(["cargo", "build"])

    print("Starting Leader...")
    leader = subprocess.Popen(["./target/debug/turbo-leader"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    
    workers = []
    print("Starting 4 Workers...")
    for i in range(4):
        w = subprocess.Popen(["./target/debug/turbo-worker"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        workers.append(w)


    try:
        print("Waiting for worker to register (10s)...")
        time.sleep(10)

        print("Submitting 20 jobs...")
        results = []
        
        from concurrent.futures import ThreadPoolExecutor, as_completed
        
        def submit_job(i):
            payload = {
                "id": "00000000-0000-0000-0000-000000000000",
                "language": "bash",
                "code": f"echo 'Job {i}'",
                "timeout_seconds": 5
            }
            
            try:
                response = requests.post("http://localhost:3001/submit", json=payload, timeout=120)
                if response.status_code == 200:
                    res_json = response.json()
                    return (i, res_json['worker_id'], "success")
                else:
                    return (i, None, f"Failed with {response.status_code}")
            except Exception as e:
                return (i, None, f"Request failed: {e}")
        
        # Submit all jobs in parallel
        with ThreadPoolExecutor(max_workers=20) as executor:
            futures = [executor.submit(submit_job, i) for i in range(20)]
            
            for future in as_completed(futures):
                i, worker_id, status = future.result()
                if status == "success":
                    print(f"Job {i}: Executed by {worker_id}")
                    results.append(worker_id)
                else:
                    print(f"Job {i}: {status}")

        # Calculate distribution
        from collections import Counter
        distribution = Counter(results)
        print("\n--- Job Distribution ---")
        for worker, count in distribution.items():
            print(f"Worker {worker}: {count} jobs")


    finally:
        print("Cleaning up...")
        os.kill(leader.pid, signal.SIGTERM)
        for i, w in enumerate(workers):
            os.kill(w.pid, signal.SIGTERM)
            print(f"\n--- Worker {i+1} Logs ---")
            print(w.stderr.read().decode())
        
        print("\n--- Leader Logs ---")
        print(leader.stderr.read().decode())


if __name__ == "__main__":
    run_test()
