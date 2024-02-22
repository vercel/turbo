# Setup
  $ . ${TESTDIR}/../../../helpers/setup_integration_test.sh persistent_dependencies/4-cross-workspace

# Workspace Graph
# - app-a depends on pkg-a
# Task Graph:
# app-a#dev
# └── pkg-a#dev
  $ ${TURBO} run dev
    x invalid persistent task configuration
  
  Error:   x "pkg-a#dev" is a persistent task, "app-a#dev" cannot depend on it
  
  [1]
