# API v3

Piston exposes an API for managing packages and executing user-defined code.

The API is broken in to 2 main sections - packages and jobs.

The API is exposed from the container, by default on port 2000, at `/api/v3/`.

All inputs are validated, and if an error occurs, a 4xx or 5xx status code is returned.
In this case, a JSON payload is sent back containing the error message as `message`

## Runtimes

### `GET /api/v2/runtimes`

> [!NOTE]
> The runtimes endpoint remains the same as v2.

Returns a list of available languages, including the version, runtime and aliases.

#### Response

-   `[].language`: Name of the language
-   `[].version`: Version of the runtime
-   `[].aliases`: List of alternative names that can be used for the language
-   `[].runtime` (_optional_): Name of the runtime used to run the langage, only provided if alternative runtimes exist for the language

#### Example

```
GET /api/v2/runtimes
```

```json
HTTP/1.1 200 OK
Content-Type: application/json

[
  {
    "language": "bash",
    "version": "5.1.0",
    "aliases": ["sh"]
  },
  {
    "language": "javascript",
    "version": "15.10.0",
    "aliases": ["node-javascript", "node-js", "javascript", "js"],
    "runtime": "node"
  }
]
```

## Execute

### `POST /api/v3/execute`

Runs the given code using the given runtime against multiple testcases. For compiled languages, the code is compiled once and executed sequentially for each testcase.

#### Request

-   `language`: Name or alias of a language listed in [runtimes](#runtimes)
-   `version`: SemVer version selector of a language listed in [runtimes](#runtimes)
-   `files`: An array of files which should be uploaded into the job context
-   `files[].name` (_optional_): Name of file to be written, if none a random name is picked
-   `files[].content`: Content of file to be written
-   `files[].encoding` (_optional_): The encoding scheme used for the file content. One of `base64`, `hex` or `utf8`. Defaults to `utf8`.
-   `testcases`: An array of testcase objects to execute.
-   `testcases[].id`: Unique identifier for the testcase.
-   `testcases[].input`: Text to pass into stdin of the program for this specific testcase.
-   `testcases[].expectedOutput`: The expected stdout string, or an array of acceptable stdout strings.
-   `args` (_optional_): Arguments to pass to the program. Defaults to none
-   `run_timeout` (_optional_): The maximum allowed time in milliseconds for the compile stage to finish before bailing out.
-   `compile_timeout` (_optional_): The maximum allowed time in milliseconds for the run stage to finish before bailing out.
-   `compile_memory_limit` (_optional_): The maximum amount of memory the compile stage is allowed to use in bytes.
-   `run_memory_limit` (_optional_): The maximum amount of memory the run stage is allowed to use in bytes.

#### Response

-   `language`: Name (not alias) of the runtime used
-   `version`: Version of the used runtime
-   `compile` (_optional_): Results from the compile stage, only provided if the runtime has a compile stage
    -   `stdout`: stdout from compile stage process
    -   `stderr`: stderr from compile stage process
    -   `output`: stdout and stderr combined
    -   `code`: Exit code from compile process
    -   `signal`: Signal from compile process
-   `testcases`: Array of results for each testcase
    -   `id`: The ID of the testcase
    -   `input`: The input provided for this testcase
    -   `expectedOutput`: The expected output provided
    -   `actualOutput`: The actual stdout produced by the code
    -   `passed`: Boolean indicating if `actualOutput` matched `expectedOutput`
    -   `run_details`: Detailed execution stats for this specific run
        -   `stdout`: stdout from run stage process
        -   `stderr`: stderr from run stage process
        -   `code`: Exit code from run process
        -   `signal`: Signal from run process
        -   `memory`: Memory usage in bytes
        -   `cpu_time`: CPU time usage in milliseconds
        -   `wall_time`: Wall clock time usage in milliseconds

#### Example

```json
POST /api/v3/execute
Content-Type: application/json

{
  "language": "python",
  "version": "3.12.0",
  "files": [
    {
      "name": "main.py",
      "content": "import sys\nprint(f'Hello {sys.stdin.read().strip()}!')"
    }
  ],
  "testcases": [
    {
      "id": "1",
      "input": "User",
      "expectedOutput": "Hello User!"
    },
    {
      "id": "2",
      "input": "World",
      "expectedOutput": "Hello World!"
    }
  ]
}
```

```json
HTTP/1.1 200 OK
Content-Type: application/json

{
  "testcases": [
    {
      "id": "1",
      "input": "User",
      "expectedOutput": "Hello User!",
      "actualOutput": "Hello User!\n",
      "passed": true,
      "run_details": {
        "stdout": "Hello User!\n",
        "stderr": "",
        "code": 0,
        "signal": null,
        "memory": 4536000,
        "cpu_time": 9,
        "wall_time": 32
      }
    },
    {
      "id": "2",
      "input": "World",
      "expectedOutput": "Hello World!",
      "actualOutput": "Hello World!\n",
      "passed": true,
      "run_details": {
        "stdout": "Hello World!\n",
        "stderr": "",
        "code": 0,
        "signal": null,
        "memory": 4924000,
        "cpu_time": 17,
        "wall_time": 8
      }
    }
  ],
  "language": "python",
  "version": "3.12.0"
}
```

## Packages

> [!NOTE]
> Package management endpoints are shared with v2.

### `GET /api/v2/packages`

Returns a list of all possible packages, and whether their installation status.

### `POST /api/v2/packages`

Install the given package.

### `DELETE /api/v2/packages`

Uninstall the given package.
