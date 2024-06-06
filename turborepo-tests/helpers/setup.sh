#!/usr/bin/env bash

THIS_DIR=$(dirname "${BASH_SOURCE[0]}")
MONOREPO_ROOT_DIR="${THIS_DIR}/../.."

if [[ "${OSTYPE}" == "msys" ]]; then
  EXT=".exe"
else
  EXT=""
fi

# disable the first-run telemetry message
export TURBO_TELEMETRY_MESSAGE_DISABLED=1
export TURBO=${MONOREPO_ROOT_DIR}/target/debug/turbo${EXT}
TURBO=${MONOREPO_ROOT_DIR}/target/debug/turbo${EXT}
