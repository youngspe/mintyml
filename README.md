<img src="https://raw.githubusercontent.com/youngspe/mintyml/main/assets/mintyml-logo.svg" title="MinTyML Logo">
<h2>What is MinTyML?</h2>
<p>MinTyML (from <dfn><ins>Min</ins>imalist H<ins>TML</ins></dfn>) is an alternative HTML syntax intended for writing documents.</p>
<h3>Principles</h3>
<p>This markup language is designed with the following principles in mind:</p>
<ol>
  <li>Structure and formatting should be as concise as possible so the writer can focus on the text.</li>
  <li>Writing documents without complex formatting or interactivity should not require strong knowledge of HTML.</li>
  <li>Any reasonable, valid HTML should be representable in MinTyML.</li>
</ol>
<h2>Using MinTyML</h2>
<h3>Demo</h3>
<p>This <a href="https://youngspe.github.io/mintyml">web-based demo</a> shows how a MinTyML document will look.</p>
<h4>Examples:</h4>
<ul>
  <li><a href="https://youngspe.github.io/mintyml?example=table">Tables</a></li>
  <li><a href="https://youngspe.github.io/mintyml?example=list">Lists</a></li>
  <li><a href="https://youngspe.github.io/mintyml?example=formatting">Formatting</a></li>
  <li><a href="https://youngspe.github.io/mintyml?example=sections">Document structure</a></li>
</ul>
<h3>Command-line interface</h3>
<p><a href="https://github.com/youngspe/mintyml/tree/main/minty-cli">mintyml-cli</a></p>
<p>The command-line MinTyML converter can be downloaded from <a href="https://github.com/youngspe/mintyml/releases/latest">Github</a> or installed with <a href="https://crates.io/crates/mintyml-cli"><code>cargo install mintyml-cli</code></a></p>
<h3>IDE tooling</h3>
<p>Try the <a href="https://marketplace.visualstudio.com/items?itemName=youngspe.mintyml">VS Code extension</a>.</p>
<h3>Libraries</h3>
<table>
  <thead><tr><th>Language</th> <th>Package</th> <th>Install</th> <th>Doc</th></tr></thead>
  <tr>
    <td>Rust</td>
    <td><a href="https://crates.io/crates/mintyml">mintyml</a></td>
    <td><code>cargo add mintyml</code></td>
    <td><a href="https://docs.rs/mintyml/latest/mintyml/">doc</a></td>
  </tr>
  <tr>
    <td>JavaScript/TypeScript</td>
    <td><a href="https://npmjs.com/package/mintyml">mintyml</a> (compatible with NodeJS or in the browser with WebPack)</td>
    <td><code>npm install --save mintyml</code></td>
    <td></td>
  </tr>
</table>
<h2>Basic structure</h2>
<p>A MinTyML document is made up of <i>nodes</i> that correspond to HTML constructs.</p>
<h3>Paragraph</h3>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>Lorem ipsum dolor sit amet,&NewLine;consectetur adipiscing elit.&NewLine;&NewLine;Sed do eiusmod tempor incididunt&NewLine;ut labore et dolore magna aliqua.</code></pre></td>
    <td>
      <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
      <p>Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
    </td>
  </tr>
  <caption>Paragraphs separated by empty lines.</caption>
</table></figure></aside>
<p>The simplest node is the <i>paragraph</i>. A paragraph is made up of one or more consecutive lines of plain text. Paragraphs are separated from each other by empty lines and cannot be nested inside other paragraphs.</p>
<h3>Selector</h3>
<p>Some elements may include a <i>selector</i>, which describes the type and attributes of the element. It's syntactically very similar to a CSS selector.</p>
<p>A selector is made up of the following components (each of which is optional):</p>
<dl>
  <dt>tag name</dt>
  <dd>Indicates the type of the element, and must be placed at the beginning of the selector. For example, <code>a</code> to create a link (also called an <i>anchor</i>). A wildcard (<code>*</code>) is the same as not including a tag name at all, which signals that the element type should be inferred from context.</dd>
  <dt>class</dt>
  <dd>A class name, preceded by a dot (<code>.</code>). Classes are used by CSS to apply styles to certain elements. For example, <code>.external</code>, which one might use to indicate a link points to another domain. A selector can have multiple classes like so: <code>.class1.class2</code></dd>
  <dt>id</dt>
  <dd>An identifier for this specific element, preceded by a hash <code>#</code>. For example, <code>#example-link</code>. No element should have the same ID as another within the same document. Can be used by CSS to style a specific element.</dd>
  <dt>attribute</dt>
  <dd>One or more HTML attributes that provide more information about the element, surrounded by brackets (<code>[ ]</code>). For example, <code>[href=http://example.com/]</code> sets the destination of a link. Multiple attributes are separated by a space. If an attribute value contains a space or a closing bracket, it can be surrounded by single quotes (<code>' '</code>) or double quotes (<code>" "</code>). For example, <code>[href=http://example.com/ title='Example site']</code></dd>
</dl>
<p>Putting all of the components together to form a selector might look like this:</p>
<pre><code>a#example-link.external[href=http://example.com/ title='Example site']</code></pre>
<p>The above selector describes a link that:</p>
<ul>
  <li>Has the unique identifier <code>example-link</code>.</li>
  <li>Is given the <code>external</code> class.</li>
  <li>Points to <code>http://example.com/</code>.</li>
  <li>Has the title <q>Example site</q>.</li>
</ul>
<p>Most elements' selectors will not be nearly as complex as this. In fact, many elements will not need a selector at all.</p>
<p>Multiple selectors can be chained, separated by <code>&gt;</code> to nest elements together.</p>
<p>For example, <code>header&gt;h2&gt;</code> describes a <code>header</code> element with a level 2 heading inside. This can also be used to compose elements with inferred types:</p>
<ul>
  <li><code>&gt;img[src=/icon.png]&gt;</code>, when used within a section context describes a paragraph containing a single image.</li>
  <li><code>section&gt;*&gt;</code> or simply <code>section&gt;&gt;</code> describes a section with a single paragraph element.</li>
</ul>
<h3>Element</h3>
<p>Elements are components of a document that have a distinct purpose and translate directly to an HTML element.</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>This is&NewLine;a paragraph.&NewLine;footer&gt; This footer is outside the paragraph.</code></pre></td>
    <td>
      <p>This is a paragraph.</p>
      <footer>This footer is outside the paragraph.</footer>
    </td>
  </tr>
  <caption>If an element directly follows a paragraph, the paragraph ends and does not include the element.</caption>
</table></figure></aside>
<p>Some common element types (or <i>tag names</i>) that might be used in a text-centric document are:</p>
<dl>
  <dt><code>p</code> (<dfn>paragraph</dfn>)</dt>
  <dd>A block of related text. Block elements that contain section content will automatically encapsulate chunks of text in this element.</dd>
  <dt><code>h1</code>, <code>h2</code>, <code>h3</code>, <code>h4</code>, <code>h5</code>, and <code>h6</code> (<dfn>heading levels 1-6</dfn>)</dt>
  <dd>Headings implicitly organize the document into sections and subsections based on the heading level.</dd>
  <dt><code>ul</code> (<dfn>unordered list</dfn>) and <code>ol</code> (<dfn>ordered list</dfn>)</dt>
  <dd>Bulleted and numbered lists, respectively. Any paragraph or element contained within that doesn't specify a type will be inferred as <code>li</code> (<dfn>list item</dfn>). See <a href="#List-Inference">list inference</a>.</dd>
  <dt><code>table</code></dt>
  <dd>
    <p>Displays data in a grid. The most basic table is made up of <code>tr</code> (<span><dfn>table row</dfn></span>) elements. Each row contains one or more cells of type <code>th</code> (<dfn>table header</dfn>) or <code>td</code> (<dfn>table data</dfn>).</p>
    <p>Like list items, rows and data cells can be inferred. See <a href="#Table-Inference">table inference</a>.</p>
  </dd>
  <dt><code>article</code></dt>
  <dd>A complete work of written text. In some cases, there may be more than one article per document.</dd>
  <dt><code>section</code></dt>
  <dd>
    <p>A logical section of a page or document. The first heading found in the section is that section's heading. Some examples of uses for sections:</p>
    <ul>
      <li>To denote chapters in a book.</li>
      <li>To separate the introduction, body, and bibliography of a paper.</li>
      <li>To mark categories of items in a menu.</li>
    </ul>
  </dd>
  <dt><code>div</code> (<dfn>content division</dfn>)</dt>
  <dd>
    <p>A container used to group other elements. This element behaves the same as a section, but doesn't imply any logical grouping of its contents. It is often used to style or visually group one or more elements.</p>
    <p>A block declared in a section context that contains section content automatically creates a <code>div</code> if no tag name is given:</p>
    <pre><code>section {&NewLine;  {&NewLine;    This is the content of a div.&NewLine;  }&NewLine;&NewLine;  .foo {&NewLine;    This is the content of a div with the class 'foo'.&NewLine;  }&NewLine;}</code></pre>
  </dd>
</dl>
<p>
  <a href="https://developer.mozilla.org/en-US/docs/Web/HTML/Element">MDN</a>
  is an excellent resource for learning more about HTML elements.
</p>
<h4>Line Element</h4>
<p>A <i>line element</i> consists of an optional selector followed by <code>&gt;</code> and optionally another node (for example a paragraph or element).</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>footer&gt; This text is the content of the footer.&NewLine;Here's another sentence in the footer.&NewLine;&NewLine;This sentence is not part of the footer.</code></pre></td>
    <td>
      <footer>This text is the content of the footer. Here's another sentence in the footer.</footer>
    </td>
  </tr>
  <caption>A line element containing text can span multiple lines</caption>
</table></figure></aside>
<p>Line elements are generally used for nodes that contain no children, or that are meant to wrap around a single child or line of text.</p>
<p>Line elements contain the node following <code>&gt;</code> if one is present.</p>
<div class="scroll-x box"><table>
  <caption></caption>
  <thead><tr>
    <th>Source</th>
    <th>Rendered</th>
  </tr></thead>
  <tr>
    <td><pre><code>p&gt; Explicit &lt;`p`&gt; tag.</code></pre></td>
    <td>
      <p>Explicit <code>p</code> tag.</p>
    </td>
  </tr>
  <tr>
    <td><pre><code>&gt; Implicit &lt;`p`&gt; tag when placed in a&NewLine;section context.</code></pre></td>
    <td>
      <p>Implicit <code>p</code> tag when placed in a section context.</p>
    </td>
  </tr>
  <tr>
    <td><pre><code>hr&gt;</code></pre></td>
    <td>
      <hr>
    </td>
  </tr>
  <tr>
    <td><pre><code>div&gt; A div containing plain text&NewLine;(not wrapped in &lt;`p`&gt; elements).</code></pre></td>
    <td>
      <div>A div containing plain text (not wrapped in <code>p</code> elements).</div>
    </td>
  </tr>
</table></div>
<h5>Inference</h5>
<p>When no element selector is provided, a line element will become a <code>p</code> when used in a section context, or a <code>span</code> in a paragraph context. For other contexts, see <a href="#Element-Inference">element inference</a>.</p>
<h4>Block Element</h4>
<p>A <i>block element</i> consists of an optional selector followed by a pair of curly braces (<code>{ }</code>) containing zero or more nodes.</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>article {&NewLine;  Article&NewLine;  section { Section 1 }&NewLine;  section { Section 2 }&NewLine;}</code></pre></td>
    <td>
      <div class="box">
        <p>Article</p>
        <div class="box">
          <p>Section 1</p>
        </div>
        <div class="box">
          <p>Section 2</p>
        </div>
      </div>
    </td>
  </tr>
  <caption>Block elements can be nested to form the structure of the document. Borders have been added for visualization purposes.</caption>
</table></figure></aside>
<p>Blocks are generally used to define <i>containers</i>, or nodes that are meant to contain other nodes. For example, the <code>article</code>, <code>section</code>, <code>table</code>, <code>details</code>, <code>ul</code>, and <code>ol</code> elements will usually be declared with braces.</p>
<h5>Inference</h5>
<p>When no element type is specified, a block element is inferred to be a <code>div</code> in a section context, or <code>span</code> in a paragraph context. For other contexts, see <a href="#Element-Inference">element inference</a>.</p>
<h4>Line-Block Element</h4>
<p>A <i>line-block element</i> consists of an optional selector followed by <code>&gt; { ... }</code>, where the curly braces contain zero or more nodes. A line-block behaves like a multi-node line element.</p>
<aside><figure>
  <table>
    <thead><tr><th>MinTyML</th> <th>HTML</th> <th>Rendered</th></tr></thead>
    <tr>
      <td><pre><code>div {&NewLine;  Line 1&NewLine;&NewLine;  Line 2&NewLine;}</code></pre></td>
      <td><pre><code>&lt;div&gt;&NewLine;  &lt;p&gt;A&lt;/p&gt;&NewLine;  &lt;p&gt;B&lt;/p&gt;&NewLine;&lt;/div&gt;</code></pre></td>
      <td>
        <div class="result">
          <div class="box">
            <p>A</p>
            <p>B</p>
          </div>
        </div>
      </td>
    </tr>
    <tr>
      <td><pre><code>div&gt; {&NewLine;  A&NewLine;&NewLine;  B&NewLine;}</code></pre></td>
      <td><pre><code>&lt;div&gt;&NewLine;  A B&NewLine;&lt;/div&gt;</code></pre></td>
      <td>
        <div class="box">
          A
          B
        </div>
      </td>
    </tr>
  </table>
  <figcaption>A block <code>div</code> and a line-block <code>div</code>. The block separates its text contents into paragraphs while the line-block lumps all the text together.</figcaption>
</figure></aside>
<p>The crucial difference between block and line-block elements is that a line-block doesn't wrap bare text up in paragraphs or any other element.</p>
<h5>Inference</h5>
<p>Line-block elements that don't specify a type will be inferred exactly the same as a line element:</p>
<scroll-x><table>
  <caption>Block vs. line-block inference comparison</caption>
  <thead><tr><th>Example</th> <th>MinTyML</th> <th>HTML</th></tr></thead>
  <tr>
    <th>Block</th>
    <td><pre><code>div {&NewLine;  { ... }&NewLine;}</code></pre></td>
    <td><pre><code>&lt;div&gt;&NewLine;  &lt;div&gt;&NewLine;    ...&NewLine;  &lt;/div&gt;&NewLine;&lt;/div&gt;</code></pre></td>
  </tr>
  <tr>
    <th>Line-Block</th>
    <td><pre><code>div {&NewLine;  &gt; { ... }&NewLine;}</code></pre></td>
    <td><pre><code>&lt;div&gt;&NewLine;  &lt;p&gt; ... &lt;/p&gt;&NewLine;&lt;/div&gt;</code></pre></td>
  </tr>
</table></scroll-x>
<h4>Inline Element</h4>
<p>An <i>inline element</i> is an element placed inside a paragraph node. It consists of an optional node wrapped in angle brackets and parentheses: <code>&lt;( ... )&gt;</code>.</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>Click &lt;(a[href=http://example.com/]&gt; here)&gt;.</code></pre></td>
    <td>
      <p>Click <a href="http://example.com">here</a>.</p>
    </td>
  </tr>
  <caption>A link defined with an inline element.</caption>
</table></figure></aside>
<p>An inline element can contain any kind of node, including elements and paragraphs.</p>
<div class="scroll-x box"><table>
  <thead>
    <tr><th rowspan="2">Inner Node</th> <th colspan="2">Example</th></tr>
    <tr><th>MinTyML</th> <th>Rendered</th></tr>
  </thead>
  <tr>
    <th>Paragraph</th>
    <td><pre><code>&lt;( Hello, &lt;_world_&gt;! )&gt;</code></pre></td>
    <td>
      <div class="result">
        <p><span>Hello, <ins>world</ins>!</span></p>
      </div>
    </td>
  </tr>
  <tr>
    <th>Line</th>
    <td><pre><code>&lt;(button[type=button]&gt; OK)&gt;</code></pre></td>
    <td>
      <div class="result">
        <p><button type="button">OK</button></p>
      </div>
    </td>
  </tr>
  <tr>
    <th>Block</th>
    <td><pre><code>ol {&NewLine;  &gt; item 1 &lt;(ol {&NewLine;    &gt; item 2&NewLine;    &gt; item 3&NewLine;  })&gt;&NewLine;  &gt; item 4 &lt;(ol {&NewLine;    &gt; item 5&NewLine;    &gt; item 6 &lt;(ol {&NewLine;      &gt; item 7&NewLine;    })&gt;&NewLine;  })&gt;&NewLine;}</code></pre></td>
    <td>
      <div class="result">
        <ol>
          <li>item 1 <ol><li>item 2</li> <li>item 3</li></ol></li>
          <li>item 4 <ol><li>item 5</li> <li>item 6 <ol><li>item 7</li></ol></li></ol></li>
        </ol>
      </div>
    </td>
  </tr>
  <tr>
    <th>Line-block</th>
    <td><pre><code>ul&gt; &lt;(eggs)&gt; &lt;(milk)&gt; &lt;(&gt; {&NewLine;  del&gt; red&NewLine;  ins&gt; green&NewLine;&NewLine;  chilis&NewLine;})&gt;</code></pre></td>
    <td>
      <div class="result">
        <ul><li>eggs</li> <li>milk</li> <li><del>red</del> <ins>green</ins> chilis</li></ul>
      </div>
    </td>
  </tr>
</table></div>
<h3 id="Inline-Formatting">Inline Formatting</h3>
<p><i>Inline formatting</i> nodes are a set of shorthands for certain inline elements. These should cover the most common scenarios for applying formatting to a slice of a paragraph.</p>
<div class="scroll-x box"><table>
  <caption>Inline formatting nodes and equivalents</caption>
  <thead><tr>
    <th>Formatting</th>
    <th>Element</th>
    <th>Appearance</th>
  </tr></thead>
  <tr>
    <td><code>&lt;#strong#&gt;</code></td>
    <td><code>&lt;(strong&gt; strong)&gt;</code></td>
    <td><strong>strong</strong></td>
  </tr>
  <tr>
    <td><code>&lt;/emphasis/&gt;</code></td>
    <td><code>&lt;(em&gt; emphasis)&gt;</code></td>
    <td><em>emphasis</em></td>
  </tr>
  <tr>
    <td><code>&lt;_underline_&gt;</code></td>
    <td><code>&lt;(u&gt; underline)&gt;</code></td>
    <td><ins>underline</ins></td>
  </tr>
  <tr>
    <td><code>&lt;~strikethrough~&gt;</code></td>
    <td><code>&lt;(s&gt; strikethrough)&gt;</code></td>
    <td><s>strikethrough</s></td>
  </tr>
  <tr>
    <td><code>&lt;"quote"&gt;</code></td>
    <td><code>&lt;(q&gt; quote)&gt;</code></td>
    <td><q>quote</q></td>
  </tr>
  <tr>
    <td><code>&lt;`code`&gt;</code></td>
    <td><code>&lt;(code&gt; &lt;[[code]]&gt;)&gt;</code></td>
    <td><code>code</code></td>
  </tr>
</table></div>
<p>Note that <code>&lt;'code'&gt;</code> is different from the others; instead of parsing its contents as MinTyML, it reads the string as-is. This does mean inline code can't contain the terminator <code>`&gt;</code> as it would be ambiguous. To work around this, you need to use an equivalent like <code>&lt;(code&gt; \&lt;`code`\&gt;)&gt;</code> or <code>&lt;(code&gt; &lt;[[&lt;`code`&gt;]]&gt;)&gt;</code> ( see <a href="#Escape-Sequence">escape sequence</a> and <a href="#Verbatim-Segment">verbatim segment</a> ).</p>
<h3>Raw Text</h3>
<p>Sometimes you want to include text without it being interpreted in the form of MinTyML nodes. For example, if your text includes characters that normally need to be escaped.</p>
<aside><figure><table>
  <thead><tr><th>Escaped</th> <th>Plaintext Block</th> <th>Verbatim Text</th></tr></thead>
  <tr>
    <td><pre><code>3 \&lt; x \&lt; 8</code></pre></td>
    <td><pre><code>"""&NewLine;3 &lt; x &lt; 8&NewLine;"""</code></pre></td>
    <td><pre><code>&lt;[[3 &lt; x &lt; 8]]&gt;</code></pre></td>
  </tr>
  <caption>Text containing <code>&lt;</code>, <code>&gt;</code>, <code>{</code>, or <code>}</code> may be easier to read in raw text form than with escape sequences.</caption>
</table></figure></aside>
<p>It's recommended that <code>script</code> and <code>style</code> elements contain raw text to avoid conflict between MinTyML syntax and JavaScript or CSS syntax.</p>
<h4 id="Plaintext-Block">Plaintext Block</h4>
<p>A <i>plaintext block</i> is consists or zero or more lines of text surrounded by either <code>'''</code> or <code>"""</code>, where the opening and closing quotes are on their own lines. Any text on the lines between the opening and closing quotes will be interpreted as plain text and will not create any nodes. Plaintext blocks delimited by double quotes (<code>"""</code>) will have escape sequences subsituted, but those delimited by single quotes (<code>'''</code>) will not.</p>
<div class="scroll-x box"><table>
  <caption>Comparison of single-quoted and double-quoted plaintext blocks</caption>
  <thead><tr><th>Delimiter</th> <th>MinTyML</th> <th>Rendered</th></tr></thead>
  <tr>
    <th>Single quotes</th>
    <td><pre><code>'''&NewLine;Hello, \u{1F30E} &lt;/world/&gt;!&NewLine;'''</code></pre></td>
    <td>
      <div class="result">
        <p>Hello, \u{1F30E} &lt;/world/&gt;!</p>
      </div>
    </td>
  </tr>
  <tr>
    <th>Double quotes</th>
    <td><pre><code>"""&NewLine;Hello, \u{1F30E} &lt;/world/&gt;!&NewLine;"""</code></pre></td>
    <td>
      <div class="result">
        <p>Hello, 🌎 &lt;/world/&gt;!</p>
      </div>
    </td>
  </tr>
</table></div>
<p>Any indentation prior to the closing quotes' indentation level will be discarded.</p>
<h4 id="Verbatim-Segment">Verbatim Segment</h4>
<p>Inspired by XML's <code>CDATA</code> syntax, a <i>verbatim segment</i> is a string of text surrounded by <code>&lt;[[</code> and <code>]]&gt;</code>. The contents of a verbatim segment are interpreted as text and included in the document as-is. This means no nodes can be nested inside the segment and escape sequences will not be substitued.</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>&lt;[#[A verbatim segment may look like this: &lt;[[ ... ]]&gt;]#]&gt;&NewLine;&lt;[##[or like this: &lt;[#[ ... ]#]&gt;]##]&gt;&NewLine;or even like this: \&lt;\[##\[ ... \]##\]\&gt;</code></pre></td>
    <td>
      <p>A verbatim segment may look like this: &lt;[[ ... ]]&gt; or like this: &lt;[#[ ... ]#]&gt; or even like this: &lt;[##[ ... ]##]&gt;</p>
    </td>
  </tr>
  <caption>Verbatim segments using the alternate delimiters.</caption>
</table></figure></aside>
<p>The alternate delimiters <code>&lt;[#[</code> and <code>]#]&gt;</code> or <code>&lt;[##[</code> and <code>]##]&gt;</code> may also be used when case the text may contain <code>]]&gt;</code> or <code>]#]&gt;</code>.</p>
<div class="scroll-x box"><table>
  <thead><tr><th>MinTyML</th> <th>HTML</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>&lt;[[Hello, world!]]&gt;</code></pre></td>
    <td><pre><code>Hello, world!</code></pre></td>
    <td>
      <div class="result">
        <p>Hello, world!</p>
      </div>
    </td>
  </tr>
  <tr>
    <td><pre><code>pre {&NewLine;  &lt;[[Hello,&NewLine;  world!]]&gt;&NewLine;}</code></pre></td>
    <td><pre><code>&lt;pre&gt;&NewLine;  Hello,&amp;Newline;world!&NewLine;&lt;/pre&gt;</code></pre></td>
    <td>
      <div class="result">
        <pre>
          Hello,&NewLine;world!
        </pre>
      </div>
    </td>
  </tr>
  <tr>
    <td><pre><code>&lt;[[Hello,\nworld!]]&gt;</code></pre></td>
    <td><pre><code>Hello,\nworld!</code></pre></td>
    <td>
      <div class="result">
        <p>Hello,\nworld!</p>
      </div>
    </td>
  </tr>
  <tr>
    <td><pre><code>&lt;[[Hello, &lt;/world/&gt;!]]&gt;</code></pre></td>
    <td><pre><code>Hello, &amp;lt;/world/&amp;gt;!</code></pre></td>
    <td>
      <div class="result">
        <p>Hello, &lt;/world/&gt;!</p>
      </div>
    </td>
  </tr>
</table></div>
<h4>Template Interpolation Segment</h4>
<p>Segments of text that resemble some common interpolation tags for template languages will remain unchanged so the MinTyML source can be compiled to an HTML template.</p>
<p>The following delimiters mark interpolations that will be unchanged:</p>
<table>
  <thead><th>Open</th> <th>Close</th> <tr>Usage examples</tr></thead>
  <tr>
    <td><code>{{</code></td>
    <td><code>}}</code></td>
    <td>Angular, Handlebars, Liquid</td>
  </tr>
  <tr>
    <td><code>{%</code></td>
    <td><code>%}</code></td>
    <td>Liquid</td>
  </tr>
  <tr>
    <td><code>&lt;%</code></td>
    <td><code>%&gt;</code></td>
    <td>Embedded Ruby</td>
  </tr>
  <tr>
    <td><code>&lt;?</code></td>
    <td><code>?&gt;</code></td>
    <td>PHP</td>
  </tr>
</table>
<h3>Code Block</h3>
<p>A <i>code block</i> closely resembles a <a href="#Plaintext-Block">plaintext block</a>, but it begins and ends with backticks (<code>```</code>) rather than quotation marks. It differs from the single-quoted plaintext block in that the contents are wrapped in a <code>code</code> element within a <code>pre</code> element. This means it will usually be rendered in a monospace font, and whitespace will not be ignored. A code block is equivalent to a single-quoted plaintext block following <code>pre&gt;code&gt;</code></p>
<div class="scroll-x box"><table>
  <caption>Code block defined with code block syntax vs. a plaintext block</caption>
  <thead><tr><th>Code Block Source</th> <th>Plaintext Block Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>```&NewLine;function add(a, b) {&NewLine;  return a + b;&NewLine;}&NewLine;```</code></pre></td>
    <td><pre><code>pre&gt;code&gt;'''&NewLine;function add(a, b) {&NewLine;  return a + b;&NewLine;}&NewLine;'''</code></pre></td>
    <td>
      <div class="result">
        <pre><code>function add(a, b) {&NewLine;  return a + b;&NewLine;}</code></pre>
      </div>
    </td>
  </tr>
</table></div>
<h3>Comment</h3>
<p>A <i>comment</i> contains text that is visible in the source of the document. but excluded from the presentation. Comments are enclosed with <code>&lt;!</code> and <code>!&gt;</code> like so:</p>
<pre><code>&lt;! This is a comment !&gt;</code></pre>
<p>The above example would be represented in HTML with:</p>
<pre><code>&lt;!-- This is a comment --&gt;</code></pre>
<aside><figure>
  <pre><code>&lt;!&NewLine;  div {&NewLine;    foo&NewLine;    &lt;! TODO: complete this div !&gt;&NewLine;  }&NewLine;!&gt;</code></pre>
  <figcaption>Comments can be nested so nodes that already contain comments can be commented out.</figcaption>
</figure></aside>
<p>Comments can be used anywhere a node is valid, including within a paragraph:</p>
<div class="scroll-x box"><table>
  <caption>Comments within a paragraph</caption>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>Hello, &lt;! this is a comment !&gt; world!</code></pre></td>
    <td>
      <div class="result">
        <p>Hello, <!-- this is a comment --> world!</p>
      </div>
    </td>
  </tr>
  <tr>
    <td><pre><code>Hello,&lt;! this is a comment !&gt;world!</code></pre></td>
    <td>
      <div class="result">
        <p>Hello,<!-- this is a comment -->world!</p>
      </div>
    </td>
  </tr>
</table></div>
<h3 id="Escape-Sequence">Escape Sequence</h3>
<p>An <i>Escape sequence</i> begins with a backslash (<code>\</code>) and provides an alternate representation of a character.</p>
<div class="scroll-x box"><table>
  <caption>All valid escapes</caption>
  <thead><tr><th>Escape</th> <th>Output</th></tr></thead>
  <tr><td><code>\n</code></td> <td>Line feed (new line)</td></tr>
  <tr><td><code>\r</code></td> <td>Carriage return</td></tr>
  <tr><td><code>\t</code></td> <td>Tab</td></tr>
  <tr><td><code>\\</code></td> <td>Backslash (<code>\</code>)</td></tr>
  <tr><td><code>\ </code> (a space following <code>\</code>)</td> <td>Space</td></tr>
  <tr>
    <td>
      <p><code>\x</code><var>hh</var></p>
      <p>Where <var>hh</var> is a 2-digit hexadecimal number no greater than <span class="num">7F</span></p>
    </td>
    <td>
      <p>The character with ASCII number <var>hh</var>.</p>
      <p>e.g. <code>\x7B</code> becomes <q>{</q>.</p>
    </td>
  </tr>
  <tr>
    <td>
      <p><code>\u{</code><var>hex</var><code>}</code></p>
      <p>Where <var>hex</var> is a hexadecimal number between 1 and 6 digits.</p>
    </td>
    <td>
      <p>The character with unicode number <var>hex</var>.</p>
      <p>e.g. <code>\u{20AC}</code> becomes <q>€</q></p>
    </td>
  </tr>
  <tr>
    <td>
      <p><code>\</code><var>sym</var></p>
      <p>Where <var>sym</var> is any of:</p>
      <ul class="row-list">
        <li><code>&lt;</code></li>
        <li><code>&gt;</code></li>
        <li><code>{</code></li>
        <li><code>}</code></li>
        <li><code>'</code></li>
        <li><code>"</code></li>
      </ul>
    </td>
    <td>
      <p><var>sym</var></p>
      <p>e.g. <code>\&gt;</code> becomes <q>&gt;</q></p>
    </td>
  </tr>
</table></div>
<h2 id="Element-Inference">Element Inference</h2>
<h3>Context</h3>
<p>The type of an unspecified element is inferred based on <i>context</i>, which is determined by the type of the containing element.</p>
<h4>Standard Contexts</h4>
<aside><figure><table>
  <thead><tr><th>Implicit</th> <th>Explicit</th></tr></thead>
  <tr>
    <td><pre><code>section {&NewLine;  {&NewLine;    &gt; {&NewLine;      &gt; Hello, world!&NewLine;    }&NewLine;  }&NewLine;&NewLine;  Goodbye, world!&NewLine;}</code></pre></td>
    <td><pre><code>section {&NewLine;  div {&NewLine;    p&gt; {&NewLine;      span&gt; Hello, world!&NewLine;    }&NewLine;}&NewLine;&NewLine;  p&gt; Goodbye, world!&NewLine;}</code></pre></td>
  </tr>
  <caption>Implicit vs equivalent explicit types</caption>
</table></figure></aside>
<dl>
  <dt>section</dt>
  <dd>
    <p>The inside of an element that may contain paragraphs, headings, lists, tables, and structural elements like <code>header</code>, <code>footer</code>, <code>section</code>, or <code>article</code>.</p>
    <p>Elements that contain a section context include:</p>
    <ul class="row-list">
      <li><code>body</code></li>
      <li><code>main</code></li>
      <li><code>article</code></li>
      <li><code>header</code></li>
      <li><code>footer</code></li>
      <li><code>section</code></li>
      <li><code>nav</code></li>
      <li><code>aside</code></li>
      <li><code>figure</code></li>
      <li><code>dialog</code></li>
      <li><code>blockquote</code></li>
      <li><code>div</code></li>
      <li><code>template</code></li>
      <li><code>hgroup</code></li>
    </ul>
  </dd>
  <dt>paragraph</dt>
  <dd>
    <p>The inside of an element that should only contain text or items that can flow with text (like buttons or images).</p>
    <p>Elements that contain a paragraph context include but are not limited to:</p>
    <ul class="row-list">
      <li><code>p</code></li>
      <li><code>h1</code>...<code>h6</code></li>
      <li><code>span</code></li>
      <li><a href="#Inline-Formatting">inline formatting</a></li>
    </ul>
  </dd>
</dl>
<p>Some elements contain section context if declared as a block element and paragraph context if declared as a line, line-block, or inline element. This includes the following:</p>
<ul class="row-list">
  <li><code>td</code> (including when inferred as a child of <code>tr</code>)</li>
  <li><code>th</code></li>
  <li><code>li</code> (including when inferred as a child of <code>ul</code> or <code>ol</code>)</li>
  <li><code>dd</code></li>
  <li><code>figcaption</code></li>
</ul>
<div class="scroll-x box"><table>
  <caption>Inference by context and node type</caption>
  <thead>
    <tr>
      <th rowspan="3">Context</th>
      <th colspan="3">Node Type</th>
    </tr>
    <tr>
      <th colspan="2">Element</th>
      <th rowspan="2">Paragraph</th>
    </tr>
    <tr>
      <th>Line</th>
      <th>Block</th>
    </tr>
  </thead>
  <tr>
    <th>Section</th>
    <td><code>p</code></td>
    <td><code>div</code></td>
    <td><code>p</code></td>
  </tr>
  <tr>
    <th>Paragraph</th>
    <td><code>span</code></td>
    <td><code>span</code></td>
    <td>plain text</td>
  </tr>
</table></div>
<h4>Specialized Contexts</h4>
<p>Some contexts are specific to a single element type or small group of element types.</p>
<aside><figure><table>
  <thead><tr><th>Source</th> <th>Rendered</th></tr></thead>
  <tr>
    <td><pre><code>details[open] {&NewLine;  More info&NewLine;&NewLine;  This is the more detailed&NewLine;  information.&NewLine;}</code></pre></td>
    <td>
      <details class="box" open>
        <summary>More info</summary>
        <p>This is the more detailed information.</p>
      </details>
    </td>
  </tr>
  <caption><code>detail</code>'s specialized context infers the first paragraph as the summary.</caption>
</table></figure></aside>
<dl>
  <dt id="List-Inference">list</dt>
  <dd>The inside of a <code>ul</code>, <code>ol</code>, or <code>menu</code> element. Infers contents to be list items (<code>li</code>).</dd>
  <dt id="Table-Inference">table</dt>
  <dd>The inside of a <code>table</code>, <code>thead</code>, <code>tbody</code>, or <code>tfoot</code> element. Infers contents to be rows (<code>tr</code>).</dd>
  <dt>table row</dt>
  <dd>The inside of a <code>tr</code> element. Infers contents to be data cells (<code>td</code>). <strong>Note</strong>: header cells (<code>th</code>) will need their element type explicitly stated.</dd>
  <dt>details</dt>
  <dd>The inside of a <code>details</code> element. The first line, block, or paragraph is inferred to be a <code>summary</code> element. The remainder of the contents are in section context.</dd>
  <dt>fieldset</dt>
  <dd>If the <code>fieldset</code> element's first node is a paragraph, that paragraph will be inferred as a <code>legend</code>. All other nodes are in section context.</dd>
  <dt>description list</dt>
  <dd>The inside of a <code>dl</code> element. Infers line elements to be description terms (<code>dt</code>) and all other nodes to be description details (<code>dd</code>).</dd>
  <dt>links and custom elements</dt>
  <dd>If used inside a paragraph context, <code>a</code> as well as any custom element contain a paragraph context if used in a paragraph context. Otherwise contains a section context.</dd>
  <dt>others</dt>
  <dd>The following special contexts can also be found on the table below:</dd>
  <ul class="row-list">
    <li>label (<code>label</code>) <ul><li>Line elements inferred as <code>input</code></li></ul></li>
    <li>data list (<code>datalist</code>, <code>optgroup</code>) <ul><li>All children inferred as <code>option</code></li></ul></li>
    <li>select (<code>select</code>) <ul><li>Like data list, but blocks inferred as <code>optgroup</code></li></ul></li>
    <li>column group (<code>colgroup</code>) <ul><li>Infers elements as <code>col</code></li></ul></li>
    <li>image map (<code>imagemap</code>) <ul><li>Infers elements as <code>area</code></li></ul></li>
  </ul>
</dl>
<div class="scroll-x box"><table>
  <caption>Inference by context and node type</caption>
  <thead>
    <tr>
      <th rowspan="3">Context</th>
      <th colspan="3">Node Type</th>
    </tr>
    <tr>
      <th colspan="2">Element</th>
      <th rowspan="2">Paragraph</th>
    </tr>
    <tr>
      <th>Line</th>
      <th>Block</th>
    </tr>
  </thead>
  <tr>
    <th>List</th>
    <td><code>li</code></td>
    <td><code>li</code></td>
    <td><code>li</code></td>
  </tr>
  <tr>
    <th>Table</th>
    <td><code>tr</code></td>
    <td><code>tr</code></td>
    <td><code>tr</code></td>
  </tr>
  <tr>
    <th>Table Row</th>
    <td><code>td</code></td>
    <td><code>td</code></td>
    <td><code>td</code></td>
  </tr>
  <tr>
    <th>Description List</th>
    <td><code>dt</code></td>
    <td><code>dd</code></td>
    <td><code>dd</code></td>
  </tr>
  <tr>
    <th>Label</th>
    <td><code>input</code></td>
    <td><code>div</code></td>
    <td><code>p</code></td>
  </tr>
  <tr>
    <th>Select</th>
    <td><code>option</code></td>
    <td><code>optgroup</code></td>
    <td><code>option</code></td>
  </tr>
  <tr>
    <th>Data List</th>
    <td><code>option</code></td>
    <td><code>option</code></td>
    <td><code>option</code></td>
  </tr>
  <tr>
    <th>Column Group</th>
    <td><code>col</code></td>
    <td><code>col</code></td>
    <td></td>
  </tr>
  <tr>
    <th>Image Map</th>
    <td><code>area</code></td>
    <td><code>area</code></td>
    <td></td>
  </tr>
</table></div>
