use indoc::indoc;

pub const GENERATE_LLMS_TXT: &str = indoc! { "
  You need to generate an llms.txt file for a website. This file summarizes and describes the main content of the website. It includes a description of the website's structured elements and all outbound links.

  Here's a more formal the off what an llms.txt file is (_note the XML-like tags delineate specific content_):
  <llms_txt_definition>
  Background:
  Large language models (LLMs) increasingly rely on website information, but face a critical limitation: context windows are too small to handle most websites in their entirety. Converting complex HTML pages with navigation, ads, and JavaScript into LLM-friendly plain text is both difficult and imprecise. While websites serve both human readers and LLMs, the latter benefit from more concise, expert-level information gathered in a single, accessible location. This is particularly important for use cases like development environments, where LLMs need quick access to programming documentation and APIs.

  Format:
  A file following the spec contains the following sections as markdown, in the specific order:

  An H1 with the name of the project or site. This is the only required section.

  A blockquote with a short summary of the project, containing key information necessary for understanding the rest of the file.

  Zero or more markdown sections (e.g. paragraphs, lists, etc) of any type except headings, containing more detailed information about the project and how to interpret the provided files.

  Zero or more markdown sections delimited by H2 headers, containing “file lists” of URLs where further detail is available.

  Each “file list” is a markdown list, containing a required markdown hyperlink [name](url), then optionally a : and notes about the file.

  Here is a mock example:
  <example>
  # Title

  > Optional description goes here

  Optional details go here

  ## Section name

  - [Link title](https://link_url): Optional link details

  ## Optional

  - [Link title](https://link_url)
  <\\example>

  Note that the “Optional” section has a special meaning—if it’s included, the URLs provided there can be skipped if a shorter context is needed. Use it for secondary information which can often be skipped.

  Existing Standards:
  llms.txt is designed to coexist with current web standards. While sitemaps list all pages for search engines, llms.txt offers a curated overview for LLMs. It can complement robots.txt by providing context for allowed content. The file can also reference structured data markup used on the site, helping LLMs understand how to interpret this information in context.

  The approach of standardising on a path for the file follows the approach of /robots.txt and /sitemap.xml. robots.txt and llms.txt have different purposes—robots.txt is generally used to let automated tools know what access to a site is considered acceptable, such as for search indexing bots. On the other hand, llms.txt information will often be used on demand when a user explicitly requests information about a topic, such as when including a coding library’s documentation in a project, or when asking a chat bot with search functionality for information. Our expectation is that llms.txt will mainly be useful for inference, i.e. at the time a user is seeking assistance, as opposed to for training. However, perhaps if llms.txt usage becomes widespread, future training runs could take advantage of the information in llms.txt files too.

  sitemap.xml is a list of all the indexable human-readable information available on a site. This isn’t a substitute for llms.txt since it:

  Often won’t have the LLM-readable versions of pages listed.

  Doesn’t include URLs to external sites, even though they might be helpful to understand the information.

  Will generally cover documents that in aggregate will be too large to fit in an LLM context window, and will include a lot of information that isn’t necessary to understand the site.

  <\\llms_txt_definition>

  This is the HTML content of the website for which you will generate an llms.txt file for:
  <website>
  ${WEBSITE}
  <\\website>

  Output only valid markdown exactly in the described llms.txt format. Do not output any other text!
"};

pub const RETRY_GENERATE_LLMS_TXT: &str = indoc! { "
  You failed to generate a valid llms.txt file!

  From the website:
  <website>
  ${WEBSITE}
  <\\website>

  You generated:
  <output>
  ${OUTPUT}
  <\\output>

  But this is not a valid markdown llms.txt file because:
  $ERROR. Please fix the error and output a valid llms.txt file for the website."
"};


pub const UPDATE_LLMS_TXT: &str = {
  "You need to update an existing llms.txt file with recent website changes."
};

pub const RETRY_UPDATE_LLMS_TXT: &str = indoc! { "
  You failed to generate a valid llms.txt file!
  From the existing llms.txt file:
  <llms_txt>
  ${LLMS_TXT}
  <\\llms_txt>
  with the updated website:
  <website>
  ${WEBSITE}
  <\\website>
  you generated:
  <output>
  ${OUTPUT}
  <\\output>
  but it wasn't a valid markdown llms.txt file because:
  <error>
  ${ERROR}
  <\\error>
  Please fix the error and output a valid updated llms.txt file for the updated website. (Only output valid markdown. Only output the extact content of the llms.txt file. Do not output any other text!)
"};
