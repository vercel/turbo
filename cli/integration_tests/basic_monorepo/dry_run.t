Setup
  $ . ${TESTDIR}/../setup.sh
  $ . ${TESTDIR}/setup.sh $(pwd)

Check my-app#build output
$ ${TURBO} run build --dry | grep "Packages in Scope" -A 4
Packages in Scope
Name   Path          
my-app apps/my-app   
util   packages/util 
expected my-app#build:  7438505b97329a3d
#                       7e5568707032a3b9
expected util#build:    6dec18f9f767112f
  
  $ ${TURBO} run build --dry | grep "my-app#build" -A 12
  my-app#build
    Task                   = build                                                                                                                           
    Package                = my-app                                                                                                                          
    Hash                   = b78753650f3f9d5f                                                                                                                
    Cached (Local)         = false                                                                                                                           
    Cached (Remote)        = false                                                                                                                           
    Directory              = apps/my-app                                                                                                                     
    Command                = echo 'building'                                                                                                                 
    Outputs                = apple.json, banana.txt                                                                                                          
    Log File               = apps/my-app/.turbo/turbo-build.log                                                                                              
    Dependencies           =                                                                                                                                 
    Dependendents          =                                                                                                                                 
    ResolvedTaskDefinition = {"outputs":["apple.json","banana.txt"],"cache":true,"dependsOn":[],"inputs":[],"outputMode":"full","env":[],"persistent":false} 
  $ ${TURBO} run build --dry | grep "util#build" -A 12
  util#build
    Task                   = build                                                                                                  
    Package                = util                                                                                                   
    Hash                   = 620c1a1e50209cea                                                                                       
    Cached (Local)         = false                                                                                                  
    Cached (Remote)        = false                                                                                                  
    Directory              = packages/util                                                                                          
    Command                = echo 'building'                                                                                        
    Outputs                =                                                                                                        
    Log File               = packages/util/.turbo/turbo-build.log                                                                   
    Dependencies           =                                                                                                        
    Dependendents          =                                                                                                        
    ResolvedTaskDefinition = {"outputs":[],"cache":true,"dependsOn":[],"inputs":[],"outputMode":"full","env":[],"persistent":false} 

# Validate output of my-app#build task
$ ${TURBO} run build --dry=json | jq '.tasks | map(select(.taskId == "my-app#build")) | .[0]'
{
"taskId": "my-app#build",
"task": "build",
"package": "my-app",
"hash": "b78753650f3f9d5f",
"cacheState": {
"local": false,
"remote": false
},
"command": "echo 'building'",
"outputs": [
"apple.json",
"banana.txt"
],
"excludedOutputs": null,
"logFile": "apps/my-app/.turbo/turbo-build.log",
"directory": "apps/my-app",
"dependencies": [],
"dependents": [],
"resolvedTaskDefinition": {
"outputs": [
"apple.json",
"banana.txt"
],
"cache": true,
"dependsOn": [],
"inputs": [],
"outputMode": "full",
"env": [],
"persistent": false
}
}

# Validate output of util#build task
$ ${TURBO} run build --dry=json | jq '.tasks | map(select(.taskId == "util#build")) | .[0]'
{
"taskId": "util#build",
"task": "build",
"package": "util",
"hash": "620c1a1e50209cea",
"cacheState": {
"local": false,
"remote": false
},
"command": "echo 'building'",
"outputs": null,
"excludedOutputs": null,
"logFile": "packages/util/.turbo/turbo-build.log",
"directory": "packages/util",
"dependencies": [],
"dependents": [],
"resolvedTaskDefinition": {
"outputs": [],
"cache": true,
"dependsOn": [],
"inputs": [],
"outputMode": "full",
"env": [],
"persistent": false
}
}
