import DOMPurify from "dompurify";
import { marked } from "marked";

marked.setOptions({
  async: false,
  breaks: true,
  gfm: true
});

let markdownHooksBound = false;

function bindMarkdownHooksOnce() {
  if (markdownHooksBound) {
    return;
  }

  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    if (typeof Element === "undefined" || !(node instanceof Element)) {
      return;
    }

    if (node.tagName === "A") {
      const href = node.getAttribute("href") ?? "";
      if (/^\s*(javascript|data):/i.test(href)) {
        node.removeAttribute("href");
      } else if (href) {
        node.setAttribute("target", "_blank");
        node.setAttribute("rel", "noopener noreferrer nofollow");
      }
    }
  });

  markdownHooksBound = true;
}

export function renderMarkdownToHtml(markdown: string): string {
  bindMarkdownHooksOnce();
  const source = (markdown ?? "").trim();
  if (!source) {
    return "";
  }

  const rawHtml = marked.parse(source) as string;
  return DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ["style", "script", "iframe", "object", "embed", "form", "input", "button"],
    FORBID_ATTR: ["style", "onerror", "onload"]
  });
}

export function markdownToPlainText(markdown: string): string {
  const safeHtml = renderMarkdownToHtml(markdown);
  if (!safeHtml) {
    return "";
  }

  const container = document.createElement("div");
  container.innerHTML = safeHtml;
  return (container.textContent ?? "").replace(/\s+/g, " ").trim();
}
