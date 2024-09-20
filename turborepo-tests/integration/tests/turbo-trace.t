Setup
  $ . ${TESTDIR}/../../helpers/setup_integration_test.sh turbo_trace

  $ ${TURBO} query "query { file(path: \"main.ts\") { path } }"
   WARNING  query command is experimental and may change in the future
  {
    "data": {
      "file": {
        "path": "main.ts"
      }
    }
  }

  $ ${TURBO} query "query { file(path: \"main.ts\") { path, fileDependencies { items { path } } } }"
   WARNING  query command is experimental and may change in the future
  {
    "data": {
      "file": {
        "path": "main.ts",
        "fileDependencies": {
          "items": [
            {
              "path": "button.tsx"
            },
            {
              "path": "foo.js"
            },
            {
              "path": "node_modules(\/|\\\\)repeat-string(\/|\\\\)index.js" (re)
            }
          ]
        }
      }
    }
  }

  $ ${TURBO} query "query { file(path: \"button.tsx\") { path, fileDependencies { items { path } } } }"
   WARNING  query command is experimental and may change in the future
  {
    "data": {
      "file": {
        "path": "button.tsx",
        "fileDependencies": {
          "items": []
        }
      }
    }
  }

  $ ${TURBO} query "query { file(path: \"circular.ts\") { path, fileDependencies { items { path } } } }"
   WARNING  query command is experimental and may change in the future
  {
    "data": {
      "file": {
        "path": "circular.ts",
        "fileDependencies": {
          "items": [
            {
              "path": "circular2.ts"
            }
          ]
        }
      }
    }
  }

