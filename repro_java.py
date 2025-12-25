import requests
import json

url = "http://localhost:3000/api/v1/execute"
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

try:
    response = requests.post(url, json=payload)
    print(f"Status Code: {response.status_code}")
    print("Response JSON:")
    print(json.dumps(response.json(), indent=2))
except Exception as e:
    print(f"Error: {e}")
