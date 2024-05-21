Setup
  $ . ${TESTDIR}/../../../helpers/setup_integration_test.sh composable_config

This test covers:
# - `cache:false` in root, override `cache:true` in workspace
# - `cache:true` in root, override to `cache:false` in workspace
# - No `cache` config in root, override `cache:false` in workspace
# - `cache:false` in root still works if workspace has no turbo.json

# cache:false in root, override to cache:true in workspace
  $ ${TURBO} run cached-task-1 --filter=cached > tmp.log
  $ cat tmp.log
  \xe2\x80\xa2 Packages in scope: cached (esc)
  \xe2\x80\xa2 Running cached-task-1 in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
  cached:cached-task-1: cache miss, executing 21deabfcd122b4f1
=======
  cached:cached-task-1: cache miss, executing ba1ebda4e92ed987
>>>>>>> 37c3c596f1 (chore: update integration tests)
  cached:cached-task-1: 
  cached:cached-task-1: > cached-task-1
  cached:cached-task-1: > echo cached-task-1 > out/foo.min.txt
  cached:cached-task-1: 
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s+[.0-9]+m?s  (re)
  
  $ HASH=$(cat tmp.log | grep -E "cached:cached-task-1.* executing .*" | awk '{print $5}')
  $ echo $HASH
  [a-z0-9]{16} (re)
  $ tar -tf $TARGET_DIR/.turbo/cache/$HASH.tar.zst;
  apps/cached/.turbo/turbo-cached-task-1.log
  apps/cached/out/
  apps/cached/out/.keep
  apps/cached/out/foo.min.txt

# cache:true in root, override to cache:false in workspace
  $ ${TURBO} run cached-task-2 --filter=cached > tmp.log
  $ cat tmp.log
  \xe2\x80\xa2 Packages in scope: cached (esc)
  \xe2\x80\xa2 Running cached-task-2 in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
  cached:cached-task-2: cache bypass, force executing 16a7b4fae5625489
=======
  cached:cached-task-2: cache bypass, force executing 5a4465328db0477f
>>>>>>> 37c3c596f1 (chore: update integration tests)
  cached:cached-task-2: 
  cached:cached-task-2: > cached-task-2
  cached:cached-task-2: > echo cached-task-2 > out/foo.min.txt
  cached:cached-task-2: 
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s+[.0-9]+m?s  (re)
  
  $ HASH=$(cat tmp.log | grep -E "cached:cached-task-2.* executing .*" | awk '{print $6}')
  $ echo $HASH
  [a-z0-9]{16} (re)
  $ test -f $TARGET_DIR/.turbo/cache/$HASH.tar.zst;
  [1]

no `cache` config in root, cache:false in workspace
  $ ${TURBO} run cached-task-3 --filter=cached > tmp.log
  $ cat tmp.log
  \xe2\x80\xa2 Packages in scope: cached (esc)
  \xe2\x80\xa2 Running cached-task-3 in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
  cached:cached-task-3: cache bypass, force executing c532469fc19ac3f9
=======
  cached:cached-task-3: cache bypass, force executing 689784cf0346aed4
>>>>>>> 37c3c596f1 (chore: update integration tests)
  cached:cached-task-3: 
  cached:cached-task-3: > cached-task-3
  cached:cached-task-3: > echo cached-task-3 > out/foo.min.txt
  cached:cached-task-3: 
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s+[.0-9]+m?s  (re)
  
  $ HASH=$(cat tmp.log | grep -E "cached:cached-task-3.* executing .*" | awk '{print $6}')
  $ echo $HASH
  [a-z0-9]{16} (re)
  $ test -f $TARGET_DIR/.turbo/cache/$HASH.tar.zst;
  [1]

cache:false in root, no turbo.json in workspace.
Note that this is run against another workspace than the other tests, because
we already have a workspace that doesn't have a config
  $ ${TURBO} run cached-task-4 --filter=missing-workspace-config > tmp.log
  $ cat tmp.log
  \xe2\x80\xa2 Packages in scope: missing-workspace-config (esc)
  \xe2\x80\xa2 Running cached-task-4 in 1 packages (esc)
  \xe2\x80\xa2 Remote caching disabled (esc)
<<<<<<< HEAD
  missing-workspace-config:cached-task-4: cache bypass, force executing 3ad36069ee49ca14
=======
  missing-workspace-config:cached-task-4: cache bypass, force executing c7b9913139d51c4e
>>>>>>> 37c3c596f1 (chore: update integration tests)
  missing-workspace-config:cached-task-4: 
  missing-workspace-config:cached-task-4: > cached-task-4
  missing-workspace-config:cached-task-4: > echo cached-task-4 > out/foo.min.txt
  missing-workspace-config:cached-task-4: 
  
   Tasks:    1 successful, 1 total
  Cached:    0 cached, 1 total
    Time:\s+[.0-9]+m?s  (re)
  
  $ HASH=$(cat tmp.log | grep -E "missing-workspace-config:cached-task-4.* executing .*" | awk '{print $6}')
  $ echo $HASH
  [a-z0-9]{16} (re)
  $ test -f $TARGET_DIR/.turbo/cache/$HASH.tar.zst;
  [1]
