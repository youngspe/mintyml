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
const index_1 = require("./index");
describe(index_1.MintymlConverter, () => {
    test('simple example', () => __awaiter(void 0, void 0, void 0, function* () {
        const target = new index_1.MintymlConverter({
            indent: 4,
        });
        const input = `\
        article {
            h1> Foo

            ul {
                > </a/>
                > b
                > c
            }
        }
        `;
        const expected = `\
<article>
    <h1>Foo</h1>
    <ul>
        <li><em>a</em></li>
        <li>b</li>
        <li>c</li>
    </ul>
</article>
`;
        const actual = yield target.convert(input);
        expect(actual).toEqual(expected);
    }));
});
