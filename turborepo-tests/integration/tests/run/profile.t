Setup
  $ . ${TESTDIR}/../../../helpers/setup_integration_test.sh

Run build and record a trace
Ignore output since we want to focus on testing the generated profile
  $ ${TURBO} build --profile=build.trace > turbo.log
  No token found for https://vercel.com/api. Run `turbo link` or `turbo login` first.
Make sure the resulting trace is valid JSON
  $ node -e "require('./build.trace')"
