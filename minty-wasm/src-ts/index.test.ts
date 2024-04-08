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
})
