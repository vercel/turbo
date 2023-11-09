#!/usr/bin/env bash

THIS_DIR=$(dirname "$(realpath "$0")")
ROOT_DIR="${THIS_DIR}/../.."

TURBO=${ROOT_DIR}/target/debug/turbo
VERSION=${ROOT_DIR}/version.txt
TMPDIR=$(mktemp -d)
