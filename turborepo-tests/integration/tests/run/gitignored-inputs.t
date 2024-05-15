Setup
  $ . ${TESTDIR}/../../../helpers/setup_integration_test.sh

Use our custom turbo config which has foo.txt as an input to the build command
  $ . ${TESTDIR}/../../../helpers/replace_turbo_json.sh $(pwd) "gitignored-inputs.json"

Create a internal.txt for the util package and add it to gitignore
This field is already part of our turbo config.
  $ echo "hello world" >> packages/util/internal.txt
  $ echo "packages/util/internal.txt" >> ${PWD}/.gitignore
  $ if [[ "$OSTYPE" == "msys" ]]; then dos2unix --quiet packages/util/internal.txt; fi
  $ git add . && git commit --quiet -m  "add internal.txt"

Some helper functions to parse the summary file
  $ source "$TESTDIR/../../../helpers/run_summary.sh"

Just run the util package, it's simpler
  $ ${TURBO} run build --filter=util --output-logs=hash-only --summarize | grep "util:build: cache"
<<<<<<< HEAD
  util:build: cache miss, executing 546eb92dc465adf3
=======
  util:build: cache miss, executing dffa41f35e6b8025
>>>>>>> 2eae5cbd82 (Update tests)

  $ FIRST=$(/bin/ls .turbo/runs/*.json | head -n1)
  $ echo $(getSummaryTaskId $FIRST "util#build") | jq -r '.inputs."internal.txt"'
  3b18e512dba79e4c8300dd08aeb37f8e728b8dad

Cleanup the runs folder so we don't have to select the correct file for the second run
  $ rm -rf .turbo/runs

Change the content of internal.txt
  $ echo "changed!" >> packages/util/internal.txt

Hash does not change, because it is gitignored
  $ ${TURBO} run build --filter=util --output-logs=hash-only --summarize | grep "util:build: cache"
<<<<<<< HEAD
  util:build: cache miss, executing 4e08438130b53119
=======
  util:build: cache miss, executing 9bddb3872b360f91
>>>>>>> 2eae5cbd82 (Update tests)

The internal.txt hash should be different from the one before
  $ SECOND=$(/bin/ls .turbo/runs/*.json | head -n1)
  $ echo $(getSummaryTaskId $SECOND "util#build") | jq -r '.inputs."internal.txt"'
  fe9ca9502b0cfe311560aa43d953a88b112609ce
