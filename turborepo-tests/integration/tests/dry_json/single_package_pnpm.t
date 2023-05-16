Setup
  $ . ${TESTDIR}/../../../helpers/setup.sh
  $ . ${TESTDIR}/../_helpers/setup_monorepo.sh $(pwd) single_package pnpm@8.0.0

  $ ${TURBO} run build --dry=json
  {
    "id": "[a-zA-Z0-9]+", (re)
    "version": "0",
    "turboVersion": "[a-z0-9\.-]+", (re)
    "globalCacheInputs": {
      "rootKey": "Buffalo buffalo Buffalo buffalo buffalo buffalo Buffalo buffalo",
      "files": {
        "package.json": "c9d031dd75d711de603996ecea3a5b9e8e64d088",
        "somefile.txt": "45b983be36b73c0788dc9cbcb76cbb80fc7bb057"
      },
      "hashOfExternalDependencies": "",
      "rootPipeline": {
        "//#build": {
          "outputs": [
            "foo"
          ],
          "cache": true,
          "dependsOn": [],
          "inputs": [],
          "outputMode": "full",
          "env": [],
          "persistent": false
        }
      }
    },
    "envMode": "infer",
    "frameworkInference": true,
    "tasks": [
      {
        "taskId": "build",
        "task": "build",
        "hash": "82a9666ce6211a46",
        "inputs": {
          ".gitignore": "6f23ff6842b5526da43ab38f4a5bf3b0158eeb42",
          "package-lock.json": "8db0df575e6509336a6719094b63eb23d2c649c1",
          "package.json": "c9d031dd75d711de603996ecea3a5b9e8e64d088",
          "somefile.txt": "45b983be36b73c0788dc9cbcb76cbb80fc7bb057",
          "turbo.json": "505752e75c10f9e7a0d2538cf8b6f0fcfb8980a0"
        },
        "hashOfExternalDependencies": "",
        "cache": {
          "local": false,
          "remote": false,
          "status": "MISS",
          "timeSaved": 0
        },
        "command": "echo 'building' \u003e foo",
        "cliArguments": [],
        "outputs": [
          "foo"
        ],
        "excludedOutputs": null,
        "logFile": ".turbo/turbo-build.log",
        "dependencies": [],
        "dependents": [],
        "resolvedTaskDefinition": {
          "outputs": [
            "foo"
          ],
          "cache": true,
          "dependsOn": [],
          "inputs": [],
          "outputMode": "full",
          "env": [],
          "persistent": false
        },
        "expandedOutputs": [],
        "framework": "\u003cNO FRAMEWORK DETECTED\u003e",
        "envMode": "loose",
        "environmentVariables": {
          "configured": [],
          "inferred": [],
          "global": [
            "VERCEL_ANALYTICS_ID="
          ],
          "passthrough": null,
          "globalPassthrough": null
        }
      }
    ],
    "user": ".*", (re)
    "scm": {
      "type": "git",
      "sha": "[a-z0-9]+", (re)
      "branch": ".+" (re)
    }
  }
