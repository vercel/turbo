(globalThis.TURBOPACK = globalThis.TURBOPACK || []).push(["output/79fb1_turbopack-tests_tests_snapshot_basic-tree-shake_export-namespace_input_e9d0cc._.js", {

"[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/export-namespace/input/lib.js [test] (ecmascript)": (({ r: __turbopack_require__, f: __turbopack_module_context__, i: __turbopack_import__, s: __turbopack_esm__, v: __turbopack_export_value__, n: __turbopack_export_namespace__, c: __turbopack_cache__, M: __turbopack_modules__, l: __turbopack_load__, j: __turbopack_dynamic__, P: __turbopack_resolve_absolute_path__, U: __turbopack_relative_url__, R: __turbopack_resolve_module_id_path__, g: global, __dirname }) => (() => {
"use strict";

__turbopack_esm__({
    "cat": ()=>cat,
    "dogRef": ()=>dogRef,
    "getChimera": ()=>getChimera,
    "initialCat": ()=>initialCat
});
let dog = "dog";
dog += "!";
console.log(dog);
function getDog() {
    return dog;
}
dog += "!";
console.log(dog);
function setDog(newDog) {
    dog = newDog;
}
dog += "!";
console.log(dog);
const dogRef = {
    initial: dog,
    get: getDog,
    set: setDog
};
let cat = "cat";
const initialCat = cat;
function getChimera() {
    return cat + dog;
}

})()),
"[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/export-namespace/input/module.js [test] (ecmascript)": (({ r: __turbopack_require__, f: __turbopack_module_context__, i: __turbopack_import__, s: __turbopack_esm__, v: __turbopack_export_value__, n: __turbopack_export_namespace__, c: __turbopack_cache__, M: __turbopack_modules__, l: __turbopack_load__, j: __turbopack_dynamic__, P: __turbopack_resolve_absolute_path__, U: __turbopack_relative_url__, R: __turbopack_resolve_module_id_path__, g: global, __dirname }) => (() => {
"use strict";

__turbopack_esm__({
    "lib": ()=>__TURBOPACK__imported__module__$5b$project$5d2f$crates$2f$turbopack$2d$tests$2f$tests$2f$snapshot$2f$basic$2d$tree$2d$shake$2f$export$2d$namespace$2f$input$2f$lib$2e$js__$5b$test$5d$__$28$ecmascript$29$__
});
var __TURBOPACK__imported__module__$5b$project$5d2f$crates$2f$turbopack$2d$tests$2f$tests$2f$snapshot$2f$basic$2d$tree$2d$shake$2f$export$2d$namespace$2f$input$2f$lib$2e$js__$5b$test$5d$__$28$ecmascript$29$__ = __turbopack_import__("[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/export-namespace/input/lib.js [test] (ecmascript)");
"__TURBOPACK__ecmascript__hoisting__location__";
;

})()),
"[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/export-namespace/input/index.js [test] (ecmascript)": (({ r: __turbopack_require__, f: __turbopack_module_context__, i: __turbopack_import__, s: __turbopack_esm__, v: __turbopack_export_value__, n: __turbopack_export_namespace__, c: __turbopack_cache__, M: __turbopack_modules__, l: __turbopack_load__, j: __turbopack_dynamic__, P: __turbopack_resolve_absolute_path__, U: __turbopack_relative_url__, R: __turbopack_resolve_module_id_path__, g: global, __dirname }) => (() => {
"use strict";

__turbopack_esm__({});
var __TURBOPACK__imported__module__$5b$project$5d2f$crates$2f$turbopack$2d$tests$2f$tests$2f$snapshot$2f$basic$2d$tree$2d$shake$2f$export$2d$namespace$2f$input$2f$module$2e$js__$5b$test$5d$__$28$ecmascript$29$__ = __turbopack_import__("[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/export-namespace/input/module.js [test] (ecmascript)");
"__TURBOPACK__ecmascript__hoisting__location__";
;
console.log(__TURBOPACK__imported__module__$5b$project$5d2f$crates$2f$turbopack$2d$tests$2f$tests$2f$snapshot$2f$basic$2d$tree$2d$shake$2f$export$2d$namespace$2f$input$2f$module$2e$js__$5b$test$5d$__$28$ecmascript$29$__["lib"].cat);

})()),
}]);

//# sourceMappingURL=79fb1_turbopack-tests_tests_snapshot_basic-tree-shake_export-namespace_input_e9d0cc._.js.map