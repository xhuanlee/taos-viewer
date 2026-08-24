import { h, type FunctionalComponent } from "vue";

interface IconProps {
  size?: number | string;
}

function createIcon(paths: string[], viewBox = "0 0 24 24"): FunctionalComponent<IconProps> {
  const Icon: FunctionalComponent<IconProps> = (props) =>
    h(
      "svg",
      {
        viewBox,
        width: props.size ?? "1em",
        height: props.size ?? "1em",
        fill: "none",
        stroke: "currentColor",
        "stroke-width": 1.7,
        "stroke-linecap": "round",
        "stroke-linejoin": "round",
        style: { flexShrink: 0 },
      },
      paths.map((d) => h("path", { d, "vector-effect": "non-scaling-stroke" }))
    );
  Icon.props = { size: [Number, String] };
  return Icon;
}

export const IconServer = createIcon([
  "M3 6a2 2 0 012-2h14a2 2 0 012 2v3a2 2 0 01-2 2H5a2 2 0 01-2-2V6z",
  "M3 15a2 2 0 012-2h14a2 2 0 012 2v3a2 2 0 01-2 2H5a2 2 0 01-2-2v-3z",
  "M7 7.5h.01",
  "M7 16.5h.01",
]);

export const IconDatabase = createIcon([
  "M12 2c4.42 0 8 1.34 8 3s-3.58 3-8 3-8-1.34-8-3 3.58-3 8-3z",
  "M4 5v14c0 1.66 3.58 3 8 3s8-1.34 8-3V5",
  "M4 12c0 1.66 3.58 3 8 3s8-1.34 8-3",
]);

export const IconTable = createIcon([
  "M5 4h14a2 2 0 012 2v12a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2z",
  "M3 10h18",
  "M10 10v10",
]);

export const IconStable = createIcon([
  "M7 2h10",
  "M9 5h6",
  "M5 8h14a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2v-8a2 2 0 012-2z",
  "M3 14h18",
  "M10 14v6",
]);

export const IconView = createIcon([
  "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z",
  "M12 15a3 3 0 100-6 3 3 0 000 6z",
]);

export const IconPlay = createIcon(["M7 5.5l11 6.5-11 6.5v-13z"]);

export const IconPlus = createIcon(["M12 5v14", "M5 12h14"]);

export const IconRefresh = createIcon([
  "M21 12a9 9 0 11-2.64-6.36",
  "M21 3v6h-6",
]);

export const IconTrash = createIcon([
  "M3 6h18",
  "M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2",
  "M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6",
  "M10 11v6",
  "M14 11v6",
]);

export const IconPencil = createIcon([
  "M17 3a2.83 2.83 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z",
]);

export const IconTerminal = createIcon([
  "M4 5h16a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V7a2 2 0 012-2z",
  "M6 9l3 3-3 3",
  "M12 15h6",
]);

export const IconSun = createIcon([
  "M12 16a4 4 0 100-8 4 4 0 000 8z",
  "M12 2v2",
  "M12 20v2",
  "M4.93 4.93l1.41 1.41",
  "M17.66 17.66l1.41 1.41",
  "M2 12h2",
  "M20 12h2",
  "M4.93 19.07l1.41-1.41",
  "M17.66 6.34l1.41-1.41",
]);

export const IconMoon = createIcon([
  "M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z",
]);

export const IconBolt = createIcon(["M13 2L3 14h7l-1 8 11-14h-8l1-6z"]);

export const IconPlug = createIcon([
  "M9 7V3",
  "M15 7V3",
  "M6 7h12v4a6 6 0 01-6 6 6 6 0 01-6-6V7z",
  "M12 17v4",
]);

export const IconGrid = createIcon([
  "M4 4h6v6H4V4z",
  "M14 4h6v6h-6V4z",
  "M4 14h6v6H4v-6z",
  "M14 14h6v6h-6v-6z",
]);

export const IconColumns = createIcon([
  "M5 4h2v16H5V4z",
  "M11 4h2v16h-2V4z",
  "M17 4h2v16h-2V4z",
]);

export const IconX = createIcon(["M6 6l12 12", "M18 6L6 18"]);

export const IconCopy = createIcon([
  "M9 9h10a2 2 0 012 2v10a2 2 0 01-2 2H9a2 2 0 01-2-2V11a2 2 0 012-2z",
  "M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1",
]);

export const IconChevronLeft = createIcon(["M15 6l-6 6 6 6"]);
export const IconChevronRight = createIcon(["M9 6l6 6-6 6"]);

export const IconFilter = createIcon([
  "M22 3H2l8 9.46V19l4 2v-8.54L22 3z",
]);
