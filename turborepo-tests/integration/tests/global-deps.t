Setup
  $ . ${TESTDIR}/../../helpers/setup_integration_test.sh global_deps

Run a build
  $ ${TURBO} build -F my-app --output-logs=hash-only
  \xe2\x80\xa2 Packages in scope: my-app (esc)
  \xe2\x80\xa2 Running build in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
<<<<<<< HEAD
  my-app:build: cache miss, executing beb8106f9ebe42f9
=======
  my-app:build: cache miss, executing 4dd187491a8e9350
>>>>>>> 2eae5cbd82 (Update tests)
=======
  my-app:build: cache miss, executing 8ce6d80ebce687e0
>>>>>>> 37c3c596f1 (chore: update integration tests)
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s*[\.0-9]+m?s  (re)
  

  $ echo "new text" > global_deps/foo.txt
  $ ${TURBO} build -F my-app --output-logs=hash-only
  \xe2\x80\xa2 Packages in scope: my-app (esc)
  \xe2\x80\xa2 Running build in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
<<<<<<< HEAD
  my-app:build: cache miss, executing ab899db87390362e
=======
  my-app:build: cache miss, executing 0ba77ba531e887b6
>>>>>>> 2eae5cbd82 (Update tests)
=======
  my-app:build: cache miss, executing dca16ac82b573af3
>>>>>>> 37c3c596f1 (chore: update integration tests)
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s*[\.0-9]+m?s  (re)
  
