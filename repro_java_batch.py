import requests
import json

url = "http://localhost:3000/api/v1/execute"
payload = {
    "language": "java",
    "version": "25.0.1",
    "files": [
        {
            "name": "Main.java",
            "content": """
import java.util.Scanner;

public class Main {
    public static void main(String[] args) {
        Scanner scanner = new Scanner(System.in);
        if (scanner.hasNextLine()) {
            String line = scanner.nextLine();
            System.out.println("Hello " + line);
        } else {
            System.out.println("Hello World");
        }
    }
}
""",
            "encoding": "utf8"
        }
    ],
    "testcases": [
        {
            "id": "1",
            "input": "Alice",
            "expected_output": "Hello Alice"
        },
        {
            "id": "2",
            "input": "Bob",
            "expected_output": "Hello Bob"
        }
    ]
}

try:
    response = requests.post(url, json=payload)
    print(f"Status Code: {response.status_code}")
    print("Response JSON:")
    print(json.dumps(response.json(), indent=2))
except Exception as e:
    print(f"Error: {e}")
