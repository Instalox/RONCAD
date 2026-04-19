// Icons — minimal line-art, Phosphor-inspired, 16px baseline.
// Always use stroke="currentColor" so they pick up CSS color.

const Icon = ({ path, size = 14, stroke = 1.5, fill = "none", children }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor"
       strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round">
    {children || <path d={path} />}
  </svg>
);

const I = {
  Cube: (p) => (
    <Icon {...p}>
      <path d="M8 1.5 L14 4.5 V11.5 L8 14.5 L2 11.5 V4.5 Z" />
      <path d="M2 4.5 L8 7.5 L14 4.5" />
      <path d="M8 7.5 V14.5" />
    </Icon>
  ),
  Plus: (p) => <Icon {...p}><path d="M8 3.5V12.5 M3.5 8H12.5" /></Icon>,
  Save: (p) => <Icon {...p}><path d="M3 3h8l2 2v8H3z M5 3v4h6V3 M5 13v-4h6v4" /></Icon>,
  Undo: (p) => <Icon {...p}><path d="M3 7h7a3 3 0 0 1 0 6H6 M3 7l2.5-2.5 M3 7l2.5 2.5" /></Icon>,
  Redo: (p) => <Icon {...p}><path d="M13 7H6a3 3 0 0 0 0 6h4 M13 7l-2.5-2.5 M13 7l-2.5 2.5" /></Icon>,
  Fit: (p) => <Icon {...p}><path d="M3 6V3h3 M13 6V3h-3 M3 10v3h3 M13 10v3h-3" /></Icon>,
  Gear: (p) => (
    <Icon {...p}>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1.5v2 M8 12.5v2 M1.5 8h2 M12.5 8h2 M3.5 3.5l1.4 1.4 M11.1 11.1l1.4 1.4 M12.5 3.5l-1.4 1.4 M4.9 11.1l-1.4 1.4" />
    </Icon>
  ),
  Chevron: (p) => <Icon {...p}><path d="M4 6l4 4 4-4" /></Icon>,
  ChevronRight: (p) => <Icon {...p}><path d="M6 4l4 4-4 4" /></Icon>,
  ChevronDown: (p) => <Icon {...p}><path d="M4 6l4 4 4-4" /></Icon>,
  Sketch: (p) => <Icon {...p}><circle cx="8" cy="8" r="4.5" /><path d="M4 8h8" /></Icon>,
  Folder: (p) => <Icon {...p}><path d="M2 4.5a1 1 0 0 1 1-1h3l1.5 1.5H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1Z" /></Icon>,
  Box: (p) => <Icon {...p}><rect x="3" y="3" width="10" height="10" rx="0.5" /></Icon>,
  Origin: (p) => <Icon {...p}><circle cx="8" cy="8" r="2" /><path d="M8 1v3 M8 12v3 M1 8h3 M12 8h3" /></Icon>,
  Plane: (p) => <Icon {...p}><path d="M3 5l5 -1.5 5 1.5v6l-5 1.5 -5 -1.5Z M3 5l5 1.5 5 -1.5 M8 6.5v6.5" /></Icon>,
  Line: (p) => <Icon {...p}><path d="M3 13 L13 3" /><circle cx="3" cy="13" r="1.3" fill="currentColor" /><circle cx="13" cy="3" r="1.3" fill="currentColor" /></Icon>,
  Rect: (p) => <Icon {...p}><rect x="2.5" y="4" width="11" height="8" /></Icon>,
  Circle: (p) => <Icon {...p}><circle cx="8" cy="8" r="5" /></Icon>,
  Arc: (p) => <Icon {...p}><path d="M3 12 A 8 8 0 0 1 13 12" /><circle cx="3" cy="12" r="1" fill="currentColor" /><circle cx="13" cy="12" r="1" fill="currentColor" /></Icon>,
  Fillet: (p) => <Icon {...p}><path d="M3 3v5 A5 5 0 0 0 8 13 h5" /></Icon>,
  Dim: (p) => <Icon {...p}><path d="M2.5 6v5 M13.5 6v5 M3 8.5h10 M3 8.5l1.5 -1.2 M3 8.5l1.5 1.2 M13 8.5l-1.5 -1.2 M13 8.5l-1.5 1.2" /></Icon>,
  Text: (p) => <Icon {...p}><path d="M3 4h10 M8 4v9 M6 13h4" /></Icon>,
  Select: (p) => <Icon {...p}><path d="M3 2 L3 12 L6 9.5 L8 13 L9.5 12 L7.5 8.5 L11.5 8 Z" /></Icon>,
  Pan: (p) => <Icon {...p}><path d="M5 7.5V4a1 1 0 0 1 2 0v4 M7 7V3a1 1 0 0 1 2 0v5 M9 7V4a1 1 0 0 1 2 0v6 M11 7V5.5a1 1 0 0 1 2 0V10.5A3.5 3.5 0 0 1 9.5 14H8.5A3 3 0 0 1 5.5 11L4 8.5a1 1 0 0 1 1.5 -1.3Z" /></Icon>,
  Extrude: (p) => <Icon {...p}><path d="M3 11 h6 v-6 h-6 z M9 5 l3 -3 M3 5 l3 -3 h6 M9 11 l3 -3 V2" /></Icon>,
  Rows: (p) => <Icon {...p}><path d="M2.5 4h11 M2.5 8h11 M2.5 12h11" /></Icon>,
  List: (p) => <Icon {...p}><path d="M3 4l1.5 1.5L7 3 M3 8l1.5 1.5L7 7 M3 12l1.5 1.5L7 11 M9 4.5h4 M9 8.5h4 M9 12.5h4" /></Icon>,
  Panel: (p) => <Icon {...p}><rect x="2.5" y="3" width="11" height="10" /><path d="M6 3v10" /></Icon>,
  Export: (p) => <Icon {...p}><path d="M8 2v8 M5 5l3 -3 3 3 M3 11v2a1 1 0 0 0 1 1h8a1 1 0 0 0 1 -1v-2" /></Icon>,
  Dots: (p) => <Icon {...p}><circle cx="4" cy="8" r="0.7" fill="currentColor" stroke="none"/><circle cx="8" cy="8" r="0.7" fill="currentColor" stroke="none"/><circle cx="12" cy="8" r="0.7" fill="currentColor" stroke="none"/></Icon>,
  // Constraint glyphs
  Coincident: (p) => <Icon {...p}><circle cx="8" cy="8" r="2.5" /><circle cx="8" cy="8" r="4.5" /></Icon>,
  Horizontal: (p) => <Icon {...p}><path d="M2.5 8h11" /></Icon>,
  Vertical: (p) => <Icon {...p}><path d="M8 2.5v11" /></Icon>,
  Parallel: (p) => <Icon {...p}><path d="M4 2.5L2 13.5 M12 2.5L10 13.5" /></Icon>,
  Perpendicular: (p) => <Icon {...p}><path d="M2.5 3v10 M2.5 13H13 M2.5 9H6" /></Icon>,
  Equal: (p) => <Icon {...p}><path d="M3 6h10 M3 10h10" /></Icon>,
  Tangent: (p) => <Icon {...p}><circle cx="6" cy="8" r="3.5" /><path d="M9.5 4.5 L13 11" /></Icon>,
  Search: (p) => <Icon {...p}><circle cx="7" cy="7" r="4" /><path d="M10 10l3.5 3.5" /></Icon>,
  Close: (p) => <Icon {...p}><path d="M4 4l8 8 M12 4l-8 8" /></Icon>,
  Refresh: (p) => <Icon {...p}><path d="M2.5 8a5.5 5.5 0 0 1 9.5 -3.5 M12 3v2.5h-2.5 M13.5 8a5.5 5.5 0 0 1 -9.5 3.5 M4 13v-2.5h2.5"/></Icon>,
  Grid: (p) => <Icon {...p}><path d="M2.5 2.5H13.5V13.5H2.5z M2.5 6H13.5 M2.5 9.5H13.5 M6 2.5v11 M9.5 2.5v11"/></Icon>,
  Hash: (p) => <Icon {...p}><path d="M4 3L3 13 M11 3L10 13 M2.5 6.5h11 M2.5 9.5h11" /></Icon>,
  Move: (p) => <Icon {...p}><path d="M8 2v12 M2 8h12 M8 2l-2 2 M8 2l2 2 M8 14l-2 -2 M8 14l2 -2 M2 8l2 -2 M2 8l2 2 M14 8l-2 -2 M14 8l-2 2"/></Icon>,
  Layers: (p) => <Icon {...p}><path d="M8 2 L14 5 L8 8 L2 5 Z M2 8 L8 11 L14 8 M2 11 L8 14 L14 11"/></Icon>,
  Command: (p) => <Icon {...p}><path d="M5 5 H11 V11 H5 Z M5 5 a1.5 1.5 0 1 1 0 -3 V5 M11 5 V2 a1.5 1.5 0 1 1 0 3 M5 11 a1.5 1.5 0 1 1 0 3 V11 M11 11 V14 a1.5 1.5 0 1 1 0 -3"/></Icon>,
  Target: (p) => <Icon {...p}><circle cx="8" cy="8" r="5"/><circle cx="8" cy="8" r="1.2" fill="currentColor" stroke="none"/></Icon>,
};

window.I = I;
window.Icon = Icon;
