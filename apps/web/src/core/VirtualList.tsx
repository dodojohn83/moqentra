import { ReactNode, useMemo, useRef, useState, UIEvent } from "react";

/**
 * Windowed list for large pages (R1-WEB-016). Only renders rows in the viewport.
 */
export function VirtualList<T>({
  items,
  rowHeight,
  height,
  renderRow,
  overscan = 5,
}: {
  items: T[];
  rowHeight: number;
  height: number;
  renderRow: (item: T, index: number) => ReactNode;
  overscan?: number;
}) {
  const [scrollTop, setScrollTop] = useState(0);
  const ref = useRef<HTMLDivElement>(null);

  const { start, end, offsetY, totalHeight } = useMemo(() => {
    const totalHeight = items.length * rowHeight;
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
    const visible = Math.ceil(height / rowHeight) + overscan * 2;
    const end = Math.min(items.length, start + visible);
    return { start, end, offsetY: start * rowHeight, totalHeight };
  }, [height, items.length, overscan, rowHeight, scrollTop]);

  function onScroll(e: UIEvent<HTMLDivElement>) {
    setScrollTop(e.currentTarget.scrollTop);
  }

  const slice = items.slice(start, end);

  return (
    <div
      ref={ref}
      onScroll={onScroll}
      style={{ height, overflow: "auto", position: "relative" }}
      role="list"
    >
      <div style={{ height: totalHeight, position: "relative" }}>
        <div style={{ transform: `translateY(${offsetY}px)` }}>
          {slice.map((item, i) => (
            <div
              key={start + i}
              role="listitem"
              style={{ height: rowHeight }}
            >
              {renderRow(item, start + i)}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
