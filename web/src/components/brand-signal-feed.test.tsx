import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { BrandSignalFeed } from "./brand-signal-feed";

vi.mock("../hooks/use-dashboard-data", () => ({
  useBrandSignals: vi.fn(),
}));

import { useBrandSignals } from "../hooks/use-dashboard-data";

const mockSignals = {
  items: [
    {
      id: 1,
      public_id: "sig-abc",
      signal_type: "article",
      title: "Test Article Title",
      summary: "A short summary of the article content.",
      source_url: "https://example.com/article",
      image_url: null,
      published_at: "2026-02-15T00:00:00Z",
      collected_at: "2026-02-16T00:00:00Z",
    },
    {
      id: 2,
      public_id: "sig-def",
      signal_type: "youtube_video",
      title: "YouTube Video Title",
      summary: null,
      source_url: "https://youtube.com/watch?v=abc",
      image_url: null,
      published_at: null,
      collected_at: "2026-02-17T00:00:00Z",
    },
    {
      id: 3,
      public_id: "sig-ghi",
      signal_type: "tweet",
      title: null,
      summary: "Tweet content here",
      source_url: null,
      image_url: null,
      published_at: "2026-02-14T00:00:00Z",
      collected_at: "2026-02-14T00:00:00Z",
    },
  ],
  next_cursor: null,
};

describe("BrandSignalFeed", () => {
  it("renders loading state", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html.toLowerCase()).toContain("loading");
  });

  it("renders error state", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error("fetch failed"),
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html.toLowerCase()).toContain("failed");
  });

  it("renders empty state when no signals", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: { items: [], next_cursor: null },
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html.toLowerCase()).toContain("no signals");
  });

  it("renders signal titles", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: mockSignals,
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html).toContain("Test Article Title");
    expect(html).toContain("YouTube Video Title");
  });

  it("renders signal summaries", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: mockSignals,
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html).toContain("A short summary of the article content.");
    expect(html).toContain("Tweet content here");
  });

  it("renders source link when source_url is present", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: mockSignals,
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html).toContain("https://example.com/article");
    expect(html).toContain("https://youtube.com/watch?v=abc");
  });

  it("renders article type icon", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: {
        items: [mockSignals.items[0]],
        next_cursor: null,
      },
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html).toContain("📰");
  });

  it("renders youtube type icon", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: {
        items: [mockSignals.items[1]],
        next_cursor: null,
      },
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html).toContain("▶");
  });

  it("does not render Load More when next_cursor is null", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: mockSignals,
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html.toLowerCase()).not.toContain("load more");
  });

  it("renders Load More when next_cursor is present", () => {
    vi.mocked(useBrandSignals).mockReturnValue({
      data: { ...mockSignals, next_cursor: 99 },
      isLoading: false,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandSignalFeed slug="test-brand" />);
    expect(html.toLowerCase()).toContain("load more");
  });
});
