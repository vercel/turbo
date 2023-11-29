Setup
  $ . ${TESTDIR}/../../../../helpers/setup.sh
  $ . ${TESTDIR}/../../../../helpers/setup_monorepo.sh $(pwd) single_package

Graph
  $ ${TURBO} run test --graph
  
  digraph {
  \tcompound = "true" (esc)
  \tnewrank = "true" (esc)
  \tsubgraph "root" { (esc)
  \t\t"[root] build" -> "[root] ___ROOT___" (esc)
  \t\t"[root] test" -> "[root] build" (esc)
  \t} (esc)
  }
  
