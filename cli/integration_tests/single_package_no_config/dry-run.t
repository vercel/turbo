Setup
  $ . ${TESTDIR}/../setup.sh
  $ . ${TESTDIR}/setup.sh $(pwd)

Check
  $ ${TURBO} run build --dry --single-package
  
  Tasks to Run
  build
    Task                   = build                                                                                                   
    Hash                   = e787148acf7f6e5e                                                                                        
    Cached (Local)         = false                                                                                                   
    Cached (Remote)        = false                                                                                                   
    Command                = echo 'building'                                                                                         
    Outputs                =                                                                                                         
    Log File               = .turbo/turbo-build.log                                                                                  
    Dependencies           =                                                                                                         
    Dependendents          =                                                                                                         
    ResolvedTaskDefinition = {"outputs":[],"cache":false,"dependsOn":[],"inputs":[],"outputMode":"full","env":[],"persistent":false} 

  $ ${TURBO} run build --dry=json --single-package
  {
    "tasks": [
      {
        "task": "build",
        "hash": "e787148acf7f6e5e",
        "cacheState": {
          "local": false,
          "remote": false
        },
        "command": "echo 'building'",
        "outputs": null,
        "excludedOutputs": null,
        "logFile": ".turbo/turbo-build.log",
        "dependencies": [],
        "dependents": [],
        "resolvedTaskDefinition": {
          "outputs": [],
          "cache": false,
          "dependsOn": [],
          "inputs": [],
          "outputMode": "full",
          "env": [],
          "persistent": false
        }
      }
    ]
  }
