(globalThis.TURBOPACK = globalThis.TURBOPACK || []).push(["output/a587c_tests_snapshot_basic-tree-shake_require-side-effect_input_de2841._.js", {

"[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/require-side-effect/input/lib.js [test] (ecmascript)": (({ r: __turbopack_require__, f: __turbopack_module_context__, i: __turbopack_import__, s: __turbopack_esm__, v: __turbopack_export_value__, n: __turbopack_export_namespace__, c: __turbopack_cache__, M: __turbopack_modules__, l: __turbopack_load__, j: __turbopack_dynamic__, P: __turbopack_resolve_absolute_path__, U: __turbopack_relative_url__, R: __turbopack_resolve_module_id_path__, g: global, __dirname }) => (() => {
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
"[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/require-side-effect/input/index.js [test] (ecmascript)": (function({ r: __turbopack_require__, f: __turbopack_module_context__, i: __turbopack_import__, s: __turbopack_esm__, v: __turbopack_export_value__, n: __turbopack_export_namespace__, c: __turbopack_cache__, M: __turbopack_modules__, l: __turbopack_load__, j: __turbopack_dynamic__, P: __turbopack_resolve_absolute_path__, U: __turbopack_relative_url__, R: __turbopack_resolve_module_id_path__, g: global, __dirname, m: module, e: exports, t: require }) { !function() {

const { cat } = __turbopack_require__("[project]/crates/turbopack-tests/tests/snapshot/basic-tree-shake/require-side-effect/input/lib.js [test] (ecmascript)");

}.call(this) }),
}]);

//# sourceMappingURL=a587c_tests_snapshot_basic-tree-shake_require-side-effect_input_de2841._.js.map