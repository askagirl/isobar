(function webpackUniversalModuleDefinition(root, factory) {
	if(typeof exports === 'object' && typeof module === 'object')
		module.exports = factory();
	else if(typeof define === 'function' && define.amd)
		define([], factory);
	else if(typeof exports === 'object')
		exports["nano"] = factory();
	else
		root["nano"] = factory();
})(global, function() {
return /******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// object to store loaded chunks
/******/ 	// "0" means "already loaded"
/******/ 	var installedChunks = {
/******/ 		"main": 0
/******/ 	};
/******/
/******/ 	// object to store loaded and loading wasm modules
/******/ 	var installedWasmModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/ 	// This file contains only the entry chunk.
/******/ 	// The chunk loading function for additional chunks
/******/ 	__webpack_require__.e = function requireEnsure(chunkId) {
/******/ 		var promises = [];
/******/
/******/
/******/ 		// require() chunk loading for javascript
/******/
/******/ 		// "0" is the signal for "already loaded"
/******/ 		if(installedChunks[chunkId] !== 0) {
/******/ 			var chunk = require("./" + chunkId + ".index.js");
/******/ 			var moreModules = chunk.modules, chunkIds = chunk.ids;
/******/ 			for(var moduleId in moreModules) {
/******/ 				modules[moduleId] = moreModules[moduleId];
/******/ 			}
/******/ 			for(var i = 0; i < chunkIds.length; i++)
/******/ 				installedChunks[chunkIds[i]] = 0;
/******/ 		}
/******/
/******/ 		// ReadFile + compile chunk loading for webassembly
/******/
/******/ 		var wasmModules = {"0":["./dist/nano_wasm_bg.wasm"]}[chunkId] || [];
/******/
/******/ 		wasmModules.forEach(function(wasmModuleId) {
/******/ 			var installedWasmModuleData = installedWasmModules[wasmModuleId];
/******/
/******/ 			// a Promise means "currently loading" or "already loaded".
/******/ 			promises.push(installedWasmModuleData ||
/******/ 				(installedWasmModules[wasmModuleId] = new Promise(function(resolve, reject) {
/******/ 					require('fs').readFile(require('path').resolve(__dirname, "" + {"./dist/nano_wasm_bg.wasm":"4ebef8e8809cd1d54059"}[wasmModuleId] + ".module.wasm"), function(err, buffer) {
/******/ 						if(err) return reject(err);
/******/ 						resolve(WebAssembly.compile(buffer));
/******/ 					});
/******/ 				}).then(function(module) { __webpack_require__.w[wasmModuleId] = module; }))
/******/ 			);
/******/ 		});
/******/ 		return Promise.all(promises);
/******/ 	};
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, {
/******/ 				configurable: false,
/******/ 				enumerable: true,
/******/ 				get: getter
/******/ 			});
/******/ 		}
/******/ 	};
/******/
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = function(exports) {
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/ 	// uncaught error handler for webpack runtime
/******/ 	__webpack_require__.oe = function(err) {
/******/ 		process.nextTick(function() {
/******/ 			throw err; // catch this error by using import().catch()
/******/ 		});
/******/ 	};
/******/
/******/ 	// object with all compiled WebAssembly.Modules
/******/ 	__webpack_require__.w = {};
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = "./src/index.ts");
/******/ })
/************************************************************************/
/******/ ({

/***/ "./src/index.ts":
/*!**********************!*\
  !*** ./src/index.ts ***!
  \**********************/
/*! exports provided: init */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"init\", function() { return init; });\nvar __awaiter = (undefined && undefined.__awaiter) || function (thisArg, _arguments, P, generator) {\r\n    return new (P || (P = Promise))(function (resolve, reject) {\r\n        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }\r\n        function rejected(value) { try { step(generator[\"throw\"](value)); } catch (e) { reject(e); } }\r\n        function step(result) { result.done ? resolve(result.value) : new P(function (resolve) { resolve(result.value); }).then(fulfilled, rejected); }\r\n        step((generator = generator.apply(thisArg, _arguments || [])).next());\r\n    });\r\n};\r\nvar __generator = (undefined && undefined.__generator) || function (thisArg, body) {\r\n    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;\r\n    return g = { next: verb(0), \"throw\": verb(1), \"return\": verb(2) }, typeof Symbol === \"function\" && (g[Symbol.iterator] = function() { return this; }), g;\r\n    function verb(n) { return function (v) { return step([n, v]); }; }\r\n    function step(op) {\r\n        if (f) throw new TypeError(\"Generator is already executing.\");\r\n        while (_) try {\r\n            if (f = 1, y && (t = op[0] & 2 ? y[\"return\"] : op[0] ? y[\"throw\"] || ((t = y[\"return\"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;\r\n            if (y = 0, t) op = [op[0] & 2, t.value];\r\n            switch (op[0]) {\r\n                case 0: case 1: t = op; break;\r\n                case 4: _.label++; return { value: op[1], done: false };\r\n                case 5: _.label++; y = op[1]; op = [0]; continue;\r\n                case 7: op = _.ops.pop(); _.trys.pop(); continue;\r\n                default:\r\n                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }\r\n                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }\r\n                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }\r\n                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }\r\n                    if (t[2]) _.ops.pop();\r\n                    _.trys.pop(); continue;\r\n            }\r\n            op = body.call(thisArg, _);\r\n        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }\r\n        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };\r\n    }\r\n};\r\nvar server;\r\nfunction init() {\r\n    return __awaiter(this, void 0, void 0, function () {\r\n        var nano;\r\n        return __generator(this, function (_a) {\r\n            switch (_a.label) {\r\n                case 0: return [4 /*yield*/, __webpack_require__.e(/*! import() */ 0).then(__webpack_require__.bind(null, /*! ../dist/nano_wasm */ \"./dist/nano_wasm.js\"))];\r\n                case 1:\r\n                    nano = _a.sent();\r\n                    if (!server) {\r\n                        server = nano.Server[\"new\"]();\r\n                    }\r\n                    return [2 /*return*/, { WorkTree: WorkTree }];\r\n            }\r\n        });\r\n    });\r\n}\r\nfunction request(req) {\r\n    var response = server.request(req);\r\n    if (response.type == \"Error\") {\r\n        throw new Error(response.message);\r\n    }\r\n    else {\r\n        return response;\r\n    }\r\n}\r\nvar FileType;\r\n(function (FileType) {\r\n    FileType[\"Directory\"] = \"Directory\";\r\n    FileType[\"File\"] = \"File\";\r\n})(FileType || (FileType = {}));\r\nvar FileStatus;\r\n(function (FileStatus) {\r\n    FileStatus[\"New\"] = \"New\";\r\n    FileStatus[\"Renamed\"] = \"Renamed\";\r\n    FileStatus[\"Removed\"] = \"Removed\";\r\n    FileStatus[\"Modified\"] = \"Modified\";\r\n    FileStatus[\"Unchanged\"] = \"Unchanged\";\r\n})(FileStatus || (FileStatus = {}));\r\nvar WorkTree = /** @class */ (function () {\r\n    function WorkTree(replicaId) {\r\n        this.id = request({\r\n            type: \"CreateWorkTree\",\r\n            replica_id: replicaId\r\n        }).tree_id;\r\n    }\r\n    WorkTree.getRootFileId = function () {\r\n        if (!WorkTree.rootFileId) {\r\n            WorkTree.rootFileId = request({ type: \"GetRootFileId\" }).file_id;\r\n        }\r\n        return WorkTree.rootFileId;\r\n    };\r\n    WorkTree.prototype.getVersion = function () {\r\n        return request({ tree_id: this.id, type: \"GetVersion\" }).version;\r\n    };\r\n    WorkTree.prototype.appendBaseEntries = function (baseEntries) {\r\n        return request({\r\n            type: \"AppendBaseEntries\",\r\n            tree_id: this.id,\r\n            entries: baseEntries\r\n        }).operations;\r\n    };\r\n    WorkTree.prototype.applyOps = function (operations) {\r\n        var response = request({\r\n            type: \"ApplyOperations\",\r\n            tree_id: this.id,\r\n            operations: operations\r\n        });\r\n        return response.operations;\r\n    };\r\n    WorkTree.prototype.newTextFile = function () {\r\n        var _a = request({\r\n            type: \"NewTextFile\",\r\n            tree_id: this.id\r\n        }), file_id = _a.file_id, operation = _a.operation;\r\n        return { fileId: file_id, operation: operation };\r\n    };\r\n    WorkTree.prototype.createDirectory = function (parentId, name) {\r\n        var _a = request({\r\n            type: \"CreateDirectory\",\r\n            tree_id: this.id,\r\n            parent_id: parentId,\r\n            name: name\r\n        }), file_id = _a.file_id, operation = _a.operation;\r\n        return { fileId: file_id, operation: operation };\r\n    };\r\n    WorkTree.prototype.openTextFile = function (fileId, baseText) {\r\n        var response = request({\r\n            type: \"OpenTextFile\",\r\n            tree_id: this.id,\r\n            file_id: fileId,\r\n            base_text: baseText\r\n        });\r\n        return response.buffer_id;\r\n    };\r\n    WorkTree.prototype.rename = function (fileId, newParentId, newName) {\r\n        return request({\r\n            type: \"Rename\",\r\n            tree_id: this.id,\r\n            file_id: fileId,\r\n            new_parent_id: newParentId,\r\n            new_name: newName\r\n        }).operation;\r\n    };\r\n    WorkTree.prototype.remove = function (fileId) {\r\n        return request({\r\n            type: \"Remove\",\r\n            tree_id: this.id,\r\n            file_id: fileId\r\n        }).operation;\r\n    };\r\n    WorkTree.prototype.edit = function (bufferId, ranges, newText) {\r\n        var response = request({\r\n            type: \"Edit\",\r\n            tree_id: this.id,\r\n            buffer_id: bufferId,\r\n            ranges: ranges,\r\n            new_text: newText\r\n        });\r\n        return response.operation;\r\n    };\r\n    WorkTree.prototype.changesSince = function (bufferId, version) {\r\n        return request({\r\n            type: \"ChangesSince\",\r\n            tree_id: this.id,\r\n            buffer_id: bufferId,\r\n            version: version\r\n        }).changes;\r\n    };\r\n    WorkTree.prototype.getText = function (bufferId) {\r\n        return request({\r\n            type: \"GetText\",\r\n            tree_id: this.id,\r\n            buffer_id: bufferId,\r\n        }).text;\r\n    };\r\n    WorkTree.prototype.fileIdForPath = function (path) {\r\n        return request({\r\n            type: \"FileIdForPath\",\r\n            tree_id: this.id,\r\n            path: path\r\n        }).file_id;\r\n    };\r\n    WorkTree.prototype.pathForFileId = function (id) {\r\n        return request({\r\n            type: \"PathForFileId\",\r\n            tree_id: this.id,\r\n            file_id: id\r\n        }).path;\r\n    };\r\n    WorkTree.prototype.entries = function (options) {\r\n        var showDeleted, descendInto;\r\n        if (options) {\r\n            showDeleted = options.showDeleted || false;\r\n            descendInto = options.descendInto || null;\r\n        }\r\n        else {\r\n            showDeleted = false;\r\n            descendInto = null;\r\n        }\r\n        return request({\r\n            type: \"Entries\",\r\n            tree_id: this.id,\r\n            show_deleted: showDeleted,\r\n            descend_into: descendInto\r\n        }).entries;\r\n    };\r\n    return WorkTree;\r\n}());\r\n\n\n//# sourceURL=webpack://nano/./src/index.ts?");

/***/ }),

/***/ "util":
/*!***********************!*\
  !*** external "util" ***!
  \***********************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("module.exports = require(\"util\");\n\n//# sourceURL=webpack://nano/external_%22util%22?");

/***/ })

/******/ });
});