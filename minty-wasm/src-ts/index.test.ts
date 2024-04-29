import { MintymlConverter } from './index'

describe(MintymlConverter, () => {
    test('simple example', async () => {
        const target = new MintymlConverter({
            indent: 4,
        })
        const input = `\
        article {
            h1> Foo

            ul {
                > </a/>
                > b
                > c
            }
        }
        `

        const expected = `\
<article>
    <h1>Foo</h1>
    <ul>
        <li><em>a</em></li>
        <li>b</li>
        <li>c</li>
    </ul>
</article>
`

        const actual = await target.convert(input)

        expect(actual).toEqual(expected)
    })
    test('specialTags', async () => {
        const target = new MintymlConverter({
            indent: 4,
            specialTags: {
                emphasis: 'i',
                strong: 'b',
                underline: 'ins',
                strike: 'del',
            }
        })
        const input = `\
        </foo/> <#bar#> <_baz_> <~qux~>
        `

        const expected = `\
<p><i>foo</i> <b>bar</b> <ins>baz</ins> <del>qux</del></p>
`

        const actual = await target.convert(input)

        expect(actual).toEqual(expected)
    })
})
