Setup
  $ . ${TESTDIR}/../../../helpers/setup_integration_test.sh monorepo_dependency_error
Run without --continue
  $ ${TURBO} build
  \xe2\x80\xa2 Packages in scope: my-app, other-app, some-lib (esc)
  \xe2\x80\xa2 Running build in 3 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
<<<<<<< HEAD
  some-lib:build: cache miss, executing 223a3af003231594
=======
  some-lib:build: cache miss, executing 1dd4e409589f7ed7
>>>>>>> 2eae5cbd82 (Update tests)
=======
  some-lib:build: cache miss, executing 0b994d7f0402f95c
>>>>>>> 37c3c596f1 (chore: update integration tests)
  some-lib:build: 
  some-lib:build: > build
  some-lib:build: > exit 2
  some-lib:build: 
  some-lib:build: npm ERR! Lifecycle script `build` failed with error: 
  some-lib:build: npm ERR! Error: command failed 
  some-lib:build: npm ERR!   in workspace: some-lib 
  some-lib:build: npm ERR!   at location: (.*)(\/|\\)apps(\/|\\)some-lib  (re)
  some-lib:build: ERROR: command finished with error: command \((.*)(\/|\\)apps(\/|\\)some-lib\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  some-lib#build: command \(.*(\/|\\)apps(\/|\\)some-lib\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  
   Tasks:    0 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s*[\.0-9]+m?s  (re)
  Failed:    some-lib#build
  
   ERROR  run failed: command  exited (1)
  [1]


Run without --continue, and with only errors.
  $ ${TURBO} build --output-logs=errors-only
  \xe2\x80\xa2 Packages in scope: my-app, other-app, some-lib (esc)
  \xe2\x80\xa2 Running build in 3 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
<<<<<<< HEAD
  some-lib:build: cache miss, executing 223a3af003231594
=======
  some-lib:build: cache miss, executing 1dd4e409589f7ed7
>>>>>>> 2eae5cbd82 (Update tests)
=======
  some-lib:build: cache miss, executing 0b994d7f0402f95c
>>>>>>> 37c3c596f1 (chore: update integration tests)
  some-lib:build: 
  some-lib:build: > build
  some-lib:build: > exit 2
  some-lib:build: 
  some-lib:build: npm ERR! Lifecycle script `build` failed with error: 
  some-lib:build: npm ERR! Error: command failed 
  some-lib:build: npm ERR!   in workspace: some-lib 
  some-lib:build: npm ERR!   at location: (.*)(\/|\\)apps(\/|\\)some-lib  (re)
  some-lib:build: ERROR: command finished with error: command \((.*)(\/|\\)apps(\/|\\)some-lib\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  some-lib#build: command \(.*\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  
   Tasks:    0 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s*[\.0-9]+m?s  (re)
  Failed:    some-lib#build
  
   ERROR  run failed: command  exited (1)
  [1]

Run with --continue
  $ ${TURBO} build --output-logs=errors-only --continue
  \xe2\x80\xa2 Packages in scope: my-app, other-app, some-lib (esc)
  \xe2\x80\xa2 Running build in 3 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
<<<<<<< HEAD
  some-lib:build: cache miss, executing 223a3af003231594
=======
  some-lib:build: cache miss, executing 1dd4e409589f7ed7
>>>>>>> 2eae5cbd82 (Update tests)
=======
  some-lib:build: cache miss, executing 0b994d7f0402f95c
>>>>>>> 37c3c596f1 (chore: update integration tests)
  some-lib:build: 
  some-lib:build: > build
  some-lib:build: > exit 2
  some-lib:build: 
  some-lib:build: npm ERR! Lifecycle script `build` failed with error: 
  some-lib:build: npm ERR! Error: command failed 
  some-lib:build: npm ERR!   in workspace: some-lib 
  some-lib:build: npm ERR!   at location: (.*)(\/|\\)apps(\/|\\)some-lib  (re)
  some-lib:build: command finished with error, but continuing...
<<<<<<< HEAD
<<<<<<< HEAD
  other-app:build: cache miss, executing 30cb4d864c805a98
=======
  other-app:build: cache miss, executing 31791045ed09bca5
>>>>>>> 2eae5cbd82 (Update tests)
=======
  other-app:build: cache miss, executing b99fa01691821c56
>>>>>>> 37c3c596f1 (chore: update integration tests)
  other-app:build: 
  other-app:build: > build
  other-app:build: > exit 3
  other-app:build: 
  other-app:build: npm ERR! Lifecycle script `build` failed with error: 
  other-app:build: npm ERR! Error: command failed 
  other-app:build: npm ERR!   in workspace: other-app 
  other-app:build: npm ERR!   at location: (.*)(\/|\\)apps(\/|\\)other-app  (re)
  other-app:build: command finished with error, but continuing...
  some-lib#build: command \((.*)(\/|\\)apps(\/|\\)some-lib\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  other-app#build: command \((.*)(\/|\\)apps(\/|\\)other-app\) .*npm(?:\.cmd)? run build exited \(1\) (re)
  
   Tasks:    1 successful, 3 total
  Cached:    0 cached, 3 total
    Time:\s*[\.0-9]+m?s  (re)
  Failed:    other-app#build, some-lib#build
  
   ERROR  run failed: command  exited (1)
  [1]

