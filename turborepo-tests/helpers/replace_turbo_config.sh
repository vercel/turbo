#!/usr/bin/env bash

PROJECT_DIR=$1
CONFIG_NAME=$2

THIS_DIR=$(dirname "${BASH_SOURCE[0]}")
MONOREPO_ROOT_DIR="$THIS_DIR/../.."

TURBO_CONFIGS_DIR="${MONOREPO_ROOT_DIR}/turborepo-tests/integration/tests/_fixtures/turbo-configs"

cp "${TURBO_CONFIGS_DIR}/$CONFIG_NAME" "$PROJECT_DIR/turbo.json"

# TODO: do we need to do this? we aren't editing a file, just using an existing one.
if [[ "$OSTYPE" == "msys" ]]; then
  dos2unix --quiet "$PROJECT_DIR/turbo.json"
fi

# Check if there are changes before trying to run git commit
# Since we're replacing an existing turbo.json, git commit -a should work and we
# don't need to git add anything.
if [[ $(git status --porcelain) ]]; then
  git commit --quiet -am "Use $CONFIG_NAME as turbo.json"
fi
