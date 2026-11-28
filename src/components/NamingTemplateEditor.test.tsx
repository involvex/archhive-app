// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { NamingTemplateEditor } from "./NamingTemplateEditor";
import { clearNamingPresets } from "@/lib/namingPresets";

// vitest.setup stubs document.createElement for download-anchor tests; restore
// the real implementation so RTL can mount components.
const createElementStub = document.createElement;
beforeAll(() => {
  document.createElement = Document.prototype.createElement.bind(document);
});
afterAll(() => {
  document.createElement = createElementStub as typeof document.createElement;
});

afterEach(() => {
  cleanup();
});

beforeEach(() => {
  clearNamingPresets();
  localStorage.clear();
});

describe("NamingTemplateEditor (R02)", () => {
  it("inserts a token chip into the template input", () => {
    const onChange = vi.fn();
    render(<NamingTemplateEditor value="{performer}/" onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "{title}" }));

    expect(onChange).toHaveBeenCalledWith("{performer}/{title}");
  });

  it("loads a built-in preset into the field", () => {
    const onChange = vi.fn();
    render(<NamingTemplateEditor value="" onChange={onChange} />);

    fireEvent.change(screen.getByLabelText("Load naming preset"), {
      target: { value: "builtin:date-title" },
    });

    expect(onChange).toHaveBeenCalledWith("{date}/{title}.{ext}");
  });

  it("saves a user preset and lists it", () => {
    const onChange = vi.fn();
    render(<NamingTemplateEditor value="{title}.{ext}" onChange={onChange} />);

    fireEvent.change(screen.getByLabelText("Preset name"), {
      target: { value: "Flat" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^save$/i }));

    expect(screen.getByText("Saved “Flat”.")).toBeTruthy();
    const select = screen.getByLabelText("Load naming preset") as HTMLSelectElement;
    expect([...select.options].some((o) => o.text === "Flat")).toBe(true);
  });
});
