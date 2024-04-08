"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.MintymlConverter = void 0;
let _mintyml;
// We need to know if we're running in a browser (bundled with e.g. WebPack)
// or in node.js. If we're in a browser, we import from 'pkg-web' which imports the
// .wasm file for webpack to bundle.
// In node, we assume there's no bundler and import from 'pkg-node' which the loads the file via 'fs'
const isBrowser = eval(`
    this === this.window
 || this === this.self
 || typeof require !== 'function'
`);
if (isBrowser) {
    _mintyml = require('../pkg-web/minty_wasm.js');
}
else {
    // Use eval so WebPack doesn't think it's a dependency
    _mintyml = eval('require')('../pkg-node/minty_wasm.js');
}
/** Converts MinTyML source to HTML. */
class MintymlConverter {
    constructor(options = {}) {
        var _a, _b, _c;
        this.xml = (_a = options.xml) !== null && _a !== void 0 ? _a : false;
        this.indent = (_b = options.indent) !== null && _b !== void 0 ? _b : null;
        this.completePage = (_c = options.completePage) !== null && _c !== void 0 ? _c : false;
    }
    /** Converts the given MinTyML string to HTML. */
    convert(src) {
        var _a;
        return __awaiter(this, void 0, void 0, function* () {
            const mintyml = yield _mintyml;
            try {
                return mintyml.convert(src, this.xml, (_a = this.indent) !== null && _a !== void 0 ? _a : -1, this.completePage);
            }
            catch (e) {
                const err = e;
                if (err.syntaxErrors) {
                    err.message = err.syntaxErrors.map(e => {
                        if ('expected' in e) {
                            return `Unexpected '${e.actual}'; expected ${e.expected.join(' | ')}`;
                        }
                        else {
                            return `Unexpected '${e.actual}'`;
                        }
                    }).join('\n');
                }
                throw e;
            }
        });
    }
}
exports.MintymlConverter = MintymlConverter;
