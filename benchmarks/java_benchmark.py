import argparse
import time
import requests
import concurrent.futures
import json
import statistics


def run_request(url, payload):
    start_time = time.time()
    try:
        response = requests.post(url, json=payload)
        latency = (time.time() - start_time) * 1000  # ms
        return {
            "status_code": response.status_code,
            "latency": latency,
            "success": response.status_code == 200 and response.json().get("run", {}).get("status") in ("Accepted", "SUCCESS")
        }
    except Exception as e:
        return {
            "status_code": -1,
            "latency": (time.time() - start_time) * 1000,
            "success": False,
            "error": str(e)
        }


def main():
    parser = argparse.ArgumentParser(description="Java Benchmark Turbo Server")
    parser.add_argument("--url", default="http://localhost:3000/api/v1/execute", help="Server URL")
    parser.add_argument("--concurrency", type=int, default=20, help="Number of concurrent requests")
    parser.add_argument("--requests", type=int, default=50, help="Total number of requests")
    args = parser.parse_args()

    payload = {
        "language": "java",
        "version": "25.0.1",
        "files": [
            {
                "name": "Main.java",
                "content": "public class Main { public static void main(String[] args) { System.out.println(\"Hello from Java Benchmark\"); } }",
                "encoding": "utf8"
            }
        ],
        "args": []
    }

    print(f"Starting Java benchmark against {args.url}")
    print(f" concurrency: {args.concurrency}")
    print(f" requests: {args.requests}")

    start_total = time.time()
    results = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as executor:
        futures = [executor.submit(run_request, args.url, payload) for _ in range(args.requests)]
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())

    total_time = time.time() - start_total
    successes = [r for r in results if r["success"]]
    latencies = [r["latency"] for r in successes]

    print("\nResults:")
    print(f"  Total Time: {total_time:.2f}s")
    print(f"  Throughput: {len(results) / total_time:.2f} req/s")
    print(f"  Success Rate: {len(successes)}/{len(results)} ({len(successes)/len(results)*100:.1f}%)")
    if latencies:
        print(f"  Avg Latency: {statistics.mean(latencies):.2f}ms")
        print(f"  Min Latency: {min(latencies):.2f}ms")
        print(f"  Max Latency: {max(latencies):.2f}ms")
        print(f"  P50 Latency: {statistics.median(latencies):.2f}ms")
        if len(latencies) >= 4:
            print(f"  P95 Latency: {statistics.quantiles(latencies, n=20)[18]:.2f}ms")


if __name__ == "__main__":
    main()
