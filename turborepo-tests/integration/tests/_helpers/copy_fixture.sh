#!/bin/bash
set -e

SCRIPT_DIR=$(dirname "$(realpath "$0")")
TARGET_DIR=$1
FIXTURE="_fixtures/${2-basic_monorepo}"
cp -a ${SCRIPT_DIR}/../$FIXTURE/. ${TARGET_DIR}/
