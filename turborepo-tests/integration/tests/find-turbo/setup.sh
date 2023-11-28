#!/bin/bash

SCRIPT_DIR=$(dirname ${BASH_SOURCE[0]})

TARGET_DIR=$1
FIXTURE_NAME=$2

# readlink should resolve the relative paths to the fixture so we have a canonicalized absolute path
FIXTURE_DIR="${SCRIPT_DIR}/../_fixtures/find_turbo/$FIXTURE_NAME"
FIXTURE_DIR2="${TESTDIR}/../_fixtures/find_turbo/$FIXTURE_NAME" # TESTDIR should be `turborepo-tests/integration/tests/find-turbo` here

echo "PWD: $PWD"
echo "HOME: ${HOME}"
echo "TMPDIR: $TMPDIR"
echo "BASH_SOURCE[0]: ${BASH_SOURCE[0]}"
echo "SCRIPT_DIR: ${SCRIPT_DIR}"
echo "TESTDIR: ${TESTDIR}"

echo "FIXTURE_DIR: $FIXTURE_DIR"
echo "FIXTURE_DIR2: $FIXTURE_DIR2"
echo "TARGET_DIR: $TARGET_DIR"
echo "READLINK_FIXTURE_DIR: $(readlink -f "$FIXTURE_DIR")"
echo "READLINK_FIXTURE_DIR2: $(readlink -f "$FIXTURE_DIR2")"
echo "READLINK_TARGET_DIR: $(readlink -f "$TARGET_DIR")"
echo "-----------"

DESTINATION="${TARGET_DIR}"
echo "cp cmd: cp -a ${FIXTURE_DIR}/. ${DESTINATION}/"
cp -a "${FIXTURE_DIR}/." "${DESTINATION}/"

# We need to symlink: turbo -> .pnpm/turbo@1.0.0/node_modules/turbo
# where `turbo` is the symlink
# and `.pnpm/turbo@1.0.0/node_modules/turbo` is the path to symlink to
if [[ "$OSTYPE" == "msys" && $FIXTURE_NAME == "linked" ]]; then
  # Delete the existing turbo directory or file, whatever exists there
  rm -rf node_modules/turbo

  # Let's enter the node_modules directory
  echo "entering node_modules directory"
  pushd node_modules > /dev/null || exit 1


  ######## Create the symlink
  # cmd //c mklink turbo "${PWD}\\.pnpm\\turbo@1.0.0\\node_modules\\turbo"
  # echo "running chmod on new symlink turbo"
  # chmod +rwx turbo
  # echo "running icacls on new symlink turbo"
  # cmd //c icacls "turbo /grant Everyone:(F)"

  # Use pnpx to run symlnk-dir because installing globally doesn't work with pnpm
  # TODO, should we install this as a dependency in this workspace so we can use it or
  # something else to avoid hitting the network in the middle of the test setup?
  echo "pnpx symlink-dir turbo .pnpm/turbo@1.0.0/node_modules/turbo"
  pnpx symlink-dir turbo .pnpm/turbo@1.0.0/node_modules/turbo

  # Get outta there
  echo "leaving node_modules directory"
  popd > /dev/null || exit 1

  # Make sure we got outta there.
  echo "PWD now is: $PWD"

  # Debug what we have
  echo "ls -al"
  ls -al

  echo "ls -al node_modules/"
  ls -al node_modules/

  echo "ls -al node_modules/turbo/"
  ls -al node_modules/turbo/

  echo "ls -al node_modules/turbo/../"
  ls -al node_modules/turbo/../

  echo "ls -al node_modules/turbo/../turbo-windows-64"
  ls -al node_modules/turbo/../turbo-windows-64

  echo "ls -al node_modules/turbo/../turbo-windows-64/bin"
  ls -al node_modules/turbo/../turbo-windows-64/bin
fi

# Copy fixtures to target directory.
# On Windows, we use rsync because cp isn't preserving symlinks. We could use rsync
# on all platforms, but want to limit the changes.
# if [[ "$OSTYPE" == "msys" ]]; then
#   echo "runing rsync cmd on windows"

#   REL_TARGET_DIR="$(realpath --relative-to="$PWD" "$TARGET_DIR")"
#   REL_FIXTURE_DIR="$(realpath --relative-to="$PWD" "$FIXTURE_DIR")"
#   REL_FIXTURE_DIR2="$(realpath --relative-to="$PWD" "$FIXTURE_DIR2")"

#   echo "REL_TARGET_DIR: $REL_TARGET_DIR"
#   echo "REL_FIXTURE_DIR: $REL_FIXTURE_DIR"
#   echo "REL_FIXTURE_DIR2: $REL_FIXTURE_DIR2"

#   echo "rsync -a $REL_FIXTURE_DIR2/. $REL_TARGET_DIR"
#   rsync -a "$REL_FIXTURE_DIR2/." "$REL_TARGET_DIR"


# else
#   DESTINATION="${TARGET_DIR}"
#   echo "cp cmd: cp -a ${FIXTURE_DIR}/. ${DESTINATION}/"
#   cp -a "${FIXTURE_DIR}/." "${DESTINATION}/"
# fi


# TODO: copy over the stub instead of having a duplicate in each fixture

# # These find_turbo fixtures have a pre-made node_modules directory that stubs out where the local turbo binary
# # would be located for specific package manager setups. For linux and darwin, we just put those binaries
# # into the fixture itself. For Windows platform, the binary itself needs to be a _real_ Windows binary. Instead
# # of maintaining many copies of these binaries, we keep one and move it over to the specific folder in node_modules
# # required by that fixture. This makes the fixture a bit dynamic in nature, but it's easier to maintain.
# ##
# # Note that we only _really_ need to do this when these tests are running on Windows, because that's the
# # only time they get used, but we will do it always, because the folders exist in the fixture and they shuoldn't be empty.
# WINDOWS_BIN="${SCRIPT_DIR}/../_fixtures/find_turbo/-windows-binary/turbostub.exe"

# if [[ "$FIXTURE_DIR" == "hoisted" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo-windows-arm64/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "linked" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/.pnpm/turbo-windows-64@1.0.0/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/.pnpm/turbo-windows-arm64@1.0.0/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "nested" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo/node_modules/turbo-windows-arm64/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "self" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/node_modules/turbo-windows-arm64/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "unplugged" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.yarn/unplugged/turbo-windows-64-npm-1.0.0-520925a700/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.yarn/unplugged/turbo-windows-arm64-npm-1.0.0-520925a700/node_modules/turbo-windows-arm64/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "unplugged_env_moved" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.moved/unplugged/turbo-windows-64-npm-1.0.0-520925a700/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.moved/unplugged/turbo-windows-arm64-npm-1.0.0-520925a700/node_modules/turbo-windows-arm64/bin/turbo.exe"
# elif [[ "$FIXTURE_DIR" == "unplugged_moved" ]]; then
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.moved/unplugged/turbo-windows-64-npm-1.0.0-520925a700/node_modules/turbo-windows-64/bin/turbo.exe"
#   cp "$WINDOWS_BIN"  "${TARGET_DIR}/.moved/unplugged/turbo-windows-arm64-npm-1.0.0-520925a700/node_modules/turbo-windows-arm64/bin/turbo.exe"
# fi
