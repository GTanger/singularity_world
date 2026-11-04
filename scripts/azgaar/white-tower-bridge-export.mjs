import fs from "node:fs/promises";
import {createHash} from "node:crypto";
import path from "node:path";
import {createRequire} from "node:module";
import {fileURLToPath} from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const azgaarRoot = process.env.AZGAAR_ROOT?.trim();
if (!azgaarRoot) {
  throw new Error("AZGAAR_ROOT is required and must point to the pinned Azgaar checkout");
}
const azgaarRequire = createRequire(path.join(path.resolve(azgaarRoot), "package.json"));
const {chromium} = azgaarRequire("playwright");

const rawOrigin = process.env.AZGAAR_ORIGIN?.trim() || "http://127.0.0.1:47192/Fantasy-Map-Generator/";
const origin = rawOrigin.endsWith("/") ? rawOrigin : `${rawOrigin}/`;
const outputGeography = path.resolve(
  process.env.WHITE_TOWER_OUTPUT?.trim() ||
    path.join(scriptDir, "../../data/world/white_tower_physical_geography_v1.json")
);
const evidenceDir = process.env.WHITE_TOWER_EVIDENCE_DIR?.trim()
  ? path.resolve(process.env.WHITE_TOWER_EVIDENCE_DIR)
  : null;
const outputPng = evidenceDir ? path.join(evidenceDir, "white-tower-bridge-height.png") : null;
const outputBiomePng = evidenceDir ? path.join(evidenceDir, "white-tower-bridge-biomes.png") : null;
const outputSummary = evidenceDir ? path.join(evidenceDir, "white-tower-bridge-summary.json") : null;
const packageDir = process.env.WHITE_TOWER_PACKAGE_DIR?.trim()
  ? path.resolve(process.env.WHITE_TOWER_PACKAGE_DIR)
  : null;

const packageFiles = packageDir
  ? {
      map: path.join(packageDir, "white_tower_peninsula_v1.map"),
      fullJson: path.join(packageDir, "white_tower_peninsula_v1_full.json"),
      politicalSvg: path.join(packageDir, "white_tower_peninsula_v1_political.svg"),
      politicalPng: path.join(packageDir, "white_tower_peninsula_v1_political.png"),
      biomesSvg: path.join(packageDir, "white_tower_peninsula_v1_biomes.svg"),
      biomesPng: path.join(packageDir, "white_tower_peninsula_v1_biomes.png"),
      heightmapSvg: path.join(packageDir, "white_tower_peninsula_v1_heightmap.svg"),
      heightmapPng: path.join(packageDir, "white_tower_peninsula_v1_heightmap.png"),
      canonOverlaySvg: path.join(packageDir, "white_tower_peninsula_v1_canon_overlay.svg"),
      canonOverlayPng: path.join(packageDir, "white_tower_peninsula_v1_canon_overlay.png"),
      cellsGeoJson: path.join(packageDir, "white_tower_peninsula_v1_cells.geojson"),
      routesGeoJson: path.join(packageDir, "white_tower_peninsula_v1_routes.geojson"),
      riversGeoJson: path.join(packageDir, "white_tower_peninsula_v1_rivers.geojson"),
      markersGeoJson: path.join(packageDir, "white_tower_peninsula_v1_markers.geojson"),
      zonesGeoJson: path.join(packageDir, "white_tower_peninsula_v1_zones.geojson"),
      manifest: path.join(packageDir, "manifest.json")
    }
  : null;

await fs.mkdir(path.dirname(outputGeography), {recursive: true});
if (evidenceDir) await fs.mkdir(evidenceDir, {recursive: true});
if (packageDir) await fs.mkdir(packageDir, {recursive: true});

async function saveDownload(page, outputPath, trigger) {
  const [download] = await Promise.all([
    page.waitForEvent("download", {timeout: 180_000}),
    trigger()
  ]);
  const failure = await download.failure();
  if (failure) throw new Error(`Azgaar download failed for ${outputPath}: ${failure}`);
  await download.saveAs(outputPath);
}

async function selectPreset(page, preset) {
  await page.evaluate(selectedPreset => {
    handleLayersPresetChange(selectedPreset);
    fitMapToScreen();
    scaleBar.style("display", "none");
  }, preset);
  await page.waitForTimeout(500);
}

async function exportPreset(page, preset, svgPath, pngPath) {
  await selectPreset(page, preset);
  await saveDownload(page, svgPath, () =>
    page.evaluate(() => window.Services.ExportMap.exportToSvg())
  );
  await saveDownload(page, pngPath, () =>
    page.evaluate(() => window.Services.ExportMap.exportToPng())
  );
}

async function fileReceipt(filePath) {
  const content = await fs.readFile(filePath);
  return {
    path: path.basename(filePath),
    bytes: content.byteLength,
    sha256: createHash("sha256").update(content).digest("hex")
  };
}

async function exportNativePackage(page, summary, physicalGeographyReceipt) {
  if (!packageFiles) return null;

  await page.evaluate(() => {
    mapName.value = "白塔半島";
  });

  const overlayHtml = await page.evaluate(() => {
    const ids = ["whiteTowerProofAnchors", "whiteTowerMajorHydrology"];
    const html = ids.map(id => document.getElementById(id)?.outerHTML ?? "");
    ids.forEach(id => document.getElementById(id)?.remove());
    return html;
  });

  await saveDownload(page, packageFiles.map, () =>
    page.evaluate(() => window.Services.Save.saveMap("machine"))
  );
  await saveDownload(page, packageFiles.fullJson, () =>
    page.evaluate(() => window.Services.ExportJson.exportToJson("Full"))
  );

  const geoJsonExports = [
    [packageFiles.cellsGeoJson, "saveGeoJsonCells"],
    [packageFiles.routesGeoJson, "saveGeoJsonRoutes"],
    [packageFiles.riversGeoJson, "saveGeoJsonRivers"],
    [packageFiles.markersGeoJson, "saveGeoJsonMarkers"],
    [packageFiles.zonesGeoJson, "saveGeoJsonZones"]
  ];
  for (const [outputPath, method] of geoJsonExports) {
    await saveDownload(page, outputPath, () =>
      page.evaluate(exportMethod => window.Services.ExportMap[exportMethod](), method)
    );
  }

  await exportPreset(page, "political", packageFiles.politicalSvg, packageFiles.politicalPng);
  await exportPreset(page, "biomes", packageFiles.biomesSvg, packageFiles.biomesPng);
  await exportPreset(page, "heightmap", packageFiles.heightmapSvg, packageFiles.heightmapPng);

  await page.evaluate(html => {
    const viewbox = document.querySelector("#viewbox");
    html.filter(Boolean).forEach(fragment => viewbox?.insertAdjacentHTML("beforeend", fragment));
  }, overlayHtml);
  await exportPreset(page, "heightmap", packageFiles.canonOverlaySvg, packageFiles.canonOverlayPng);

  const receipts = [];
  for (const filePath of Object.values(packageFiles).filter(filePath => filePath !== packageFiles.manifest)) {
    receipts.push(await fileReceipt(filePath));
  }
  const manifest = {
    schemaVersion: 1,
    id: "white_tower_peninsula_azgaar_package_v1",
    name: "白塔半島 Azgaar 完整作者資料包 v1",
    status: "working_proposal",
    canon: false,
    runtimeAuthority: false,
    playerFacingMap: false,
    source: summary.source,
    generated: summary.generated,
    canonAuthority: "../white_tower_topology_v1.json",
    physicalGeography: {
      ...physicalGeographyReceipt,
      path: "../white_tower_physical_geography_v1.json"
    },
    dispositions: {
      nativeAzgaarData: "retained_as_non_canon_candidates",
      acceptedWhiteTowerFacts: "overlay_and_validation_authority",
      generatedNamesPoliticsCultures: "candidate_not_canon"
    },
    files: receipts
  };
  await fs.writeFile(packageFiles.manifest, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH?.trim() || "/usr/bin/google-chrome",
  headless: true,
  args: ["--disable-gpu"]
});

const context = await browser.newContext({viewport: {width: 1600, height: 1000}});
const page = await context.newPage();
const consoleErrors = [];
const consoleWarnings = [];
const rejectedSurfaceErrors = [];
const pageErrors = [];
let captureCustomGeneration = false;

page.on("console", message => {
  const rejectedNameFallback =
    message.type() === "error" && message.text() === "Name is too short! Random name will be selected";
  if (rejectedNameFallback) {
    rejectedSurfaceErrors.push(message.text());
  } else if (message.type() === "error") {
    consoleErrors.push(message.text());
  }
  if (captureCustomGeneration && message.type() === "warning") consoleWarnings.push(message.text());
});
page.on("pageerror", error => pageErrors.push(error.message));

try {
  await page.addInitScript(() => {
    localStorage.clear();
    localStorage.setItem("template", "peninsula");
    localStorage.setItem("points", "13");
    localStorage.setItem("preset", "heightmap");
    localStorage.setItem("distanceUnit", "km");
    localStorage.setItem("heightUnit", "m");
    localStorage.setItem("mapSize", "12");
    localStorage.setItem("latitude", "25");
    localStorage.setItem("longitude", "50");
  });

  await page.goto(`${origin}?seed=white-tower-bootstrap&width=1600&height=1000`, {
    waitUntil: "domcontentloaded",
    timeout: 120_000
  });

  await page.waitForFunction(
    () => typeof pack !== "undefined" && pack?.cells?.i?.length > 0 && window.mapId,
    undefined,
    {timeout: 180_000}
  );

  captureCustomGeneration = true;
  await page.evaluate(async () => {
    const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
    const smoothstep = value => {
      const t = clamp(value, 0, 1);
      return t * t * (3 - 2 * t);
    };
    const interpolate = (value, stops) => {
      for (let index = 1; index < stops.length; index++) {
        const [x1, y1] = stops[index - 1];
        const [x2, y2] = stops[index];
        if (value <= x2) return y1 + ((y2 - y1) * (value - x1)) / (x2 - x1);
      }
      return stops.at(-1)[1];
    };
    const gaussian = (x, y, cx, cy, sx, sy) =>
      Math.exp(-(((x - cx) / sx) ** 2 + ((y - cy) / sy) ** 2) / 2);
    const distanceToSegment = (x, y, x1, y1, x2, y2) => {
      const dx = x2 - x1;
      const dy = y2 - y1;
      const lengthSquared = dx * dx + dy * dy;
      const t = clamp(((x - x1) * dx + (y - y1) * dy) / lengthSquared, 0, 1);
      return Math.hypot(x - (x1 + t * dx), y - (y1 + t * dy));
    };
    const valley = (x, y, points, width, depth) => {
      let distance = Infinity;
      for (let index = 1; index < points.length; index++) {
        distance = Math.min(distance, distanceToSegment(x, y, ...points[index - 1], ...points[index]));
      }
      return depth * Math.exp(-((distance / width) ** 2));
    };
    const blackMarshField = (x, y) => {
      const lowerRiverDistance = Math.min(
        distanceToSegment(x, y, 0.485, 0.58, 0.515, 0.69),
        distanceToSegment(x, y, 0.515, 0.69, 0.55, 0.77),
        distanceToSegment(x, y, 0.55, 0.77, 0.49, 0.89)
      );
      const riverFen = Math.exp(-((lowerRiverDistance / 0.035) ** 2));
      const westBackwater = valley(
        x,
        y,
        [[0.51, 0.69], [0.485, 0.72], [0.45, 0.755], [0.405, 0.77]],
        0.034,
        1
      );
      const eastBackwater = valley(
        x,
        y,
        [[0.535, 0.72], [0.575, 0.715], [0.615, 0.69], [0.67, 0.685]],
        0.037,
        1
      );
      const southernPools = Math.max(
        gaussian(x, y, 0.475, 0.82, 0.07, 0.045),
        gaussian(x, y, 0.57, 0.81, 0.08, 0.055)
      );
      const brokenEdge =
        0.09 * Math.sin(x * 83 + y * 37) + 0.06 * Math.sin(x * 151 - y * 61);
      const centralFen = gaussian(x, y, 0.535, 0.755, 0.085, 0.07);
      return Math.max(
        riverFen,
        westBackwater * 0.82,
        eastBackwater * 0.78,
        southernPools * 0.76,
        centralFen * 0.66
      ) + brokenEdge;
    };

    HeightmapGenerator.generate = async graph => {
      const heights = new Uint8Array(graph.points.length);

      graph.points.forEach(([pointX, pointY], index) => {
        const x = pointX / graphWidth;
        const y = pointY / graphHeight;

        // The peninsula points north and remains attached to the larger world at the south.
        const center = 0.535 + 0.018 * Math.sin((y - 0.12) * Math.PI * 2.2);
        const halfWidth = interpolate(y, [
          [0.02, 0.015],
          [0.08, 0.13],
          [0.18, 0.22],
          [0.36, 0.275],
          [0.58, 0.33],
          [0.72, 0.315],
          [0.84, 0.31],
          [0.94, 0.275],
          [1.02, 0.22]
        ]);
        const coastNoise =
          0.012 * Math.sin(y * 51 + Math.sin(y * 13) * 2) +
          0.006 * Math.sin(y * 113 + 0.7);
        const normalizedCoastDistance = Math.abs(x - center) / Math.max(halfWidth + coastNoise, 0.01);
        const northRound = smoothstep((y - 0.018) / 0.075);
        const mainLandStrength = northRound * (1 - normalizedCoastDistance);

        // Windship Island: a long rocky island west of the mainland, leaving a clear channel.
        const islandCenterX = 0.175 + 0.012 * (y - 0.34);
        const islandDistance = Math.sqrt(((x - islandCenterX) / 0.043) ** 2 + ((y - 0.36) / 0.17) ** 2);
        const islandStrength = 1 - islandDistance;

        let height = 0;
        if (mainLandStrength > 0) {
          height = 20 + 13 * smoothstep(mainLandStrength * 2.2) + 8 * (1 - y);

          // Northern Sacred Mountain is a compact summit; Green Ridge stays highland, not one ice sheet.
          height += 57 * gaussian(x, y, 0.625, 0.085, 0.038, 0.045);
          const ridgeDistance = Math.min(
            distanceToSegment(x, y, 0.65, 0.14, 0.785, 0.48),
            distanceToSegment(x, y, 0.785, 0.48, 0.79, 0.7),
            distanceToSegment(x, y, 0.79, 0.7, 0.67, 0.86),
            distanceToSegment(x, y, 0.67, 0.86, 0.54, 1.02)
          );
          const ridgeUplift = 12 + 9 * smoothstep((y - 0.34) / 0.17);
          height += ridgeUplift * Math.exp(-((ridgeDistance / 0.04) ** 2));

          // Northwest divide sends one catchment east toward Wenyao; the southwest stays broad and low.
          const westernDivideDistance = distanceToSegment(x, y, 0.255, 0.19, 0.235, 0.53);
          height += 14 * Math.exp(-((westernDivideDistance / 0.045) ** 2));
          height += 17 * gaussian(x, y, 0.31, 0.29, 0.11, 0.14);

          // Three graded source valleys meet south of Wenyao, then descend to the southern marsh.
          // Keep the branch cuts separate until the junction so their overlapping depths do not
          // flatten the whole confluence into an artificial lowland shelf.
          height -= valley(
            x,
            y,
            [[0.49, 0.17], [0.47, 0.31], [0.455, 0.445], [0.465, 0.47]],
            0.013,
            7
          );
          height -= valley(
            x,
            y,
            [[0.31, 0.29], [0.37, 0.39], [0.44, 0.445], [0.45, 0.475]],
            0.016,
            8
          );
          height -= valley(
            x,
            y,
            [[0.68, 0.27], [0.58, 0.38], [0.5, 0.44], [0.495, 0.47]],
            0.013,
            7
          );
          height -= valley(x, y, [[0.465, 0.47], [0.478, 0.5]], 0.008, 3);
          height -= valley(x, y, [[0.45, 0.475], [0.478, 0.5]], 0.008, 3);
          height -= valley(x, y, [[0.495, 0.47], [0.478, 0.5]], 0.008, 3);
          height -= 1.5 * gaussian(x, y, 0.478, 0.5, 0.014, 0.012);
          height -= valley(
            x,
            y,
            [[0.478, 0.5], [0.495, 0.57], [0.515, 0.64]],
            0.012,
            5
          );
          height -= valley(
            x,
            y,
            [[0.495, 0.57], [0.515, 0.64], [0.53, 0.76], [0.5, 1.02]],
            0.014,
            11
          );
          height += 9 * gaussian(x, y, 0.46, 0.45, 0.02, 0.017);

          // Black Marsh follows the lower river and branching backwaters instead of an oval stamp.
          height -= 7 * clamp((blackMarshField(x, y) - 0.12) / 0.88, 0, 1);
          height += 6 * gaussian(x, y, 0.48, 0.625, 0.025, 0.022);
          height = Math.max(height, 20.5);
        }

        if (islandStrength > 0) {
          height = Math.max(
            height,
            20 + 14 * smoothstep(islandStrength * 2) + 5 * gaussian(x, y, islandCenterX, 0.33, 0.026, 0.13)
          );
        }

        // Small exposed rocks at the northern mouth, without closing the harbor channel.
        height = Math.max(height, 23 * gaussian(x, y, 0.235, 0.19, 0.012, 0.018));
        height = Math.max(height, 22 * gaussian(x, y, 0.255, 0.22, 0.009, 0.012));

        heights[index] = Math.round(clamp(height, 0, 100));
      });

      return heights;
    };

    // Black Marsh is accepted geography, not a biome suggestion. Feed its trapped moisture
    // into Azgaar before river and biome calculation, while leaving the rest of the climate procedural.
    const generateRivers = Rivers.generate.bind(Rivers);
    Rivers.generate = (...args) => {
      for (const gridCellId of grid.cells.i) {
        const [pointX, pointY] = grid.points[gridCellId];
        const x = pointX / graphWidth;
        const y = pointY / graphHeight;
        const insideBlackMarsh = y >= 0.57 && y <= 0.91 && blackMarshField(x, y) >= 0.56;
        const insideCentralForest = ((x - 0.55) / 0.14) ** 2 + ((y - 0.38) / 0.1) ** 2 <= 1;
        if (insideBlackMarsh && grid.cells.h[gridCellId] >= 20) {
          grid.cells.prec[gridCellId] = Math.max(grid.cells.prec[gridCellId], 34);
        }
        if (insideCentralForest && grid.cells.h[gridCellId] >= 20) {
          grid.cells.prec[gridCellId] = Math.max(grid.cells.prec[gridCellId], 18);
        }
      }
      return generateRivers(...args);
    };

    ensureEl("templateInput").value = "whiteTowerConstrained";
    ensureEl("resolveDepressionsStepsInput").value = "1000";
    ensureEl("resolveDepressionsStepsOutput").value = "1000";
    localStorage.setItem("template", "whiteTowerConstrained");

    await new Promise(resolve => {
      window.addEventListener("map:generated", resolve, {once: true});
      regenerateMap({seed: "white-tower-peninsula-v1"});
    });

    setLayersPreset("heightmap");
    undraw();
    drawLayers();
    fitMapToScreen();
    scaleBar.style("display", "none");

    const anchors = [
      {
        id: "wt-n-sacred-mountain",
        label: "白塔",
        x: 0.625,
        y: 0.075,
        surface: "land",
        minHeight: 70,
        biomeNames: ["Glacier"]
      },
      {id: "wt-n-north-crown-sea", label: "北冕海", x: 0.2, y: 0.075, surface: "water"},
      {id: "wt-w-windship-island", label: "風舶嶼", x: 0.17, y: 0.35, surface: "land"},
      {
        id: "wt-w-natural-harbor-channel",
        label: "天然港灣道",
        x: 0.24,
        y: 0.39,
        surface: "water",
        render: false
      },
      {id: "wt-w-windship-cape", label: "風舶角", x: 0.3, y: 0.37, surface: "land"},
      {
        id: "wt-c-wenyao",
        label: "文曜",
        x: 0.46,
        y: 0.45,
        surface: "land",
        minHeight: 38,
        avoidRiver: true
      },
      {id: "wt-c-sacred-tower", label: "聖塔", x: 0.63, y: 0.31, surface: "land"},
      {id: "wt-e-green-ridge", label: "青脊山", x: 0.785, y: 0.55, surface: "land", minHeight: 50},
      {id: "wt-e-east-sea", label: "東海", x: 0.92, y: 0.54, surface: "water"},
      {
        id: "wt-c-southwest-plain",
        label: "西南原",
        x: 0.28,
        y: 0.64,
        surface: "land",
        biomeNames: ["Grassland"]
      },
      {
        id: "wt-s-south-sentinel",
        label: "南哨",
        x: 0.48,
        y: 0.625,
        surface: "land",
        minHeight: 34,
        avoidRiver: true,
        excludedBiomeNames: ["Wetland"]
      },
      {
        id: "wt-s-black-marsh",
        label: "黑沼林",
        x: 0.54,
        y: 0.74,
        surface: "land",
        biomeNames: ["Wetland"]
      },
      {
        id: "wt-s-qinglong-pass",
        label: "青籠關",
        x: 0.61,
        y: 0.9,
        surface: "land",
        minHeight: 45,
        excludedBiomeNames: ["Wetland"]
      }
    ];
    window.__whiteTowerProofAnchors = anchors;
    const namespace = "http://www.w3.org/2000/svg";
    const group = document.createElementNS(namespace, "g");
    group.id = "whiteTowerProofAnchors";
    group.setAttribute("font-family", "Noto Sans CJK TC, sans-serif");
    group.setAttribute("font-size", "16");
    group.setAttribute("font-weight", "700");
    group.setAttribute("paint-order", "stroke");
    group.setAttribute("stroke", "#f7f0dc");
    group.setAttribute("stroke-width", "4");
    group.setAttribute("stroke-linejoin", "round");
    group.setAttribute("fill", "#22201b");
    for (const {label, x, y, render = true} of anchors) {
      if (!render) continue;
      const circle = document.createElementNS(namespace, "circle");
      circle.setAttribute("cx", x * graphWidth);
      circle.setAttribute("cy", y * graphHeight);
      circle.setAttribute("r", "4");
      circle.setAttribute("stroke", "#f7f0dc");
      circle.setAttribute("stroke-width", "2");
      circle.setAttribute("fill", "#22201b");
      group.append(circle);

      const text = document.createElementNS(namespace, "text");
      text.setAttribute("x", x * graphWidth + 8);
      text.setAttribute("y", y * graphHeight - 8);
      text.textContent = label;
      group.append(text);
    }
    document.querySelector("#viewbox").append(group);
  });

  await page.waitForTimeout(1500);

  const exported = await page.evaluate(() => {
    const cells = pack.cells;
    const landCells = Array.from(cells.i).filter(index => cells.h[index] >= 20);
    const landFeatures = pack.features
      .filter(feature => feature?.type === "island" || feature?.land)
      .map(feature => ({
        id: feature.i,
        type: feature.type,
        group: feature.group,
        cells: feature.cells,
        area: feature.area
      }))
      .sort((a, b) => b.cells - a.cells);

    const anchorCellMappings = window.__whiteTowerProofAnchors.map(anchor => {
      const expectedLand = anchor.surface === "land";
      const matchesAnchorSemantics = cellId => {
        if (anchor.minHeight !== undefined && cells.h[cellId] < anchor.minHeight) return false;
        if (anchor.avoidRiver && cells.r[cellId] !== 0) return false;
        const biomeName = biomesData.name[cells.biome[cellId]];
        if (anchor.biomeNames && !anchor.biomeNames.includes(biomeName)) return false;
        if (anchor.excludedBiomeNames?.includes(biomeName)) return false;
        return true;
      };
      let nearest = null;
      let nearestDistance = Infinity;
      for (const requireSemantics of [true, false]) {
        for (const cellId of cells.i) {
          const isLand = cells.h[cellId] >= 20;
          if (isLand !== expectedLand) continue;
          if (requireSemantics && !matchesAnchorSemantics(cellId)) continue;
          const [x, y] = cells.p[cellId];
          const distance = Math.hypot(x / graphWidth - anchor.x, y / graphHeight - anchor.y);
          if (distance >= nearestDistance) continue;
          nearest = cellId;
          nearestDistance = distance;
        }
        if (nearest !== null) break;
      }
      return {
        topologyNodeRef: anchor.id,
        label: anchor.label,
        expectedSurface: anchor.surface,
        cellId: nearest,
        normalizedDistance: Number(nearestDistance.toFixed(6)),
        height: nearest === null ? null : cells.h[nearest],
        biome: nearest === null ? null : cells.biome[nearest],
        biomeName: nearest === null ? null : biomesData.name[cells.biome[nearest]],
        river: nearest === null ? null : cells.r[nearest]
      };
    });

    const summary = {
      source: {
        project: "Azgaar/Fantasy-Map-Generator",
        seed,
        heightmap: "whiteTowerConstrained",
        gridCells: grid.cells.i.length,
        packedCells: cells.i.length,
        landCells: landCells.length
      },
      generated: {
        features: pack.features.length - 1,
        landFeatures: landFeatures.slice(0, 8),
        rivers: pack.rivers.length,
        lakes: pack.features.filter(feature => feature?.type === "lake").length,
        biomes: new Set(landCells.map(index => cells.biome[index])).size,
        burgCandidates: pack.burgs.filter(burg => burg?.i && !burg.removed).length,
        routeCandidates: pack.routes.length,
        stateCandidates: pack.states.filter(state => state?.i && !state.removed).length,
        provinceCandidates: pack.provinces.filter(province => province?.i && !province.removed).length,
        cultureCandidates: pack.cultures.filter(culture => culture?.i && !culture.removed).length,
        religionCandidates: pack.religions.filter(religion => religion?.i && !religion.removed).length,
        // Azgaar 的 markers / zones 從 0 起算；id=0 是實體記錄，不是 sentinel。
        markerCandidates: pack.markers.filter(marker => Number.isInteger(marker?.i) && marker.i >= 0 && !marker.removed).length,
        zoneCandidates: pack.zones.filter(zone => Number.isInteger(zone?.i) && zone.i >= 0 && !zone.removed).length,
        goods: pack.goods.length,
        markets: pack.markets.length,
        deals: pack.deals.length
      },
      anchorCellMappings,
      constraints: {
        canonAuthority: "white_tower_topology_v1.json",
        generatedNamesAreCanon: false,
        generatedPoliticsAreCanon: false,
        playerFacingMap: false
      }
    };

    const round = (value, digits = 6) => Number(Number(value).toFixed(digits));
    const landAnchors = window.__whiteTowerProofAnchors.filter(anchor => anchor.surface === "land");
    const primaryFeatureIds = new Set(
      landFeatures
        .filter(feature => feature.cells >= 100 && ["continent", "island"].includes(feature.group))
        .map(feature => feature.id)
    );
    const physicalLandCells = landCells.filter(cellId => primaryFeatureIds.has(cells.f[cellId]));

    const elevationBand = height => {
      if (height >= 70) return "alpine";
      if (height >= 50) return "highland";
      if (height >= 35) return "upland";
      return "lowland";
    };
    const terrainClass = (biomeName, height) => {
      if (height >= 70 || biomeName === "Glacier") return "snow_and_ice";
      if (biomeName === "Wetland") return "wetland";
      if (/forest|Taiga/i.test(biomeName)) return "forest";
      if (/Grassland|Savanna/i.test(biomeName)) return "grassland";
      if (/Tundra/i.test(biomeName)) return "tundra";
      if (/Desert/i.test(biomeName)) return "open_bare_ground";
      return "mixed_ground";
    };
    const nearestLandAnchor = cellId => {
      const [pointX, pointY] = cells.p[cellId];
      const x = pointX / graphWidth;
      const y = pointY / graphHeight;
      let nearest = landAnchors[0];
      let distance = Infinity;
      for (const anchor of landAnchors) {
        const candidate = Math.hypot(x - anchor.x, y - anchor.y);
        if (candidate >= distance) continue;
        nearest = anchor;
        distance = candidate;
      }
      return nearest;
    };

    const physicalAggregateSize = 0.05;
    const cellKeys = new Map();
    for (const cellId of physicalLandCells) {
      const anchor = nearestLandAnchor(cellId);
      const [pointX, pointY] = cells.p[cellId];
      const aggregateX = Math.floor(pointX / graphWidth / physicalAggregateSize);
      const aggregateY = Math.floor(pointY / graphHeight / physicalAggregateSize);
      cellKeys.set(
        cellId,
        `${anchor.id}|${aggregateX}|${aggregateY}`
      );
    }

    const componentByCell = new Int32Array(cells.i.length);
    componentByCell.fill(-1);
    const rawComponents = [];
    for (const startCellId of physicalLandCells) {
      if (componentByCell[startCellId] !== -1) continue;
      const key = cellKeys.get(startCellId);
      const componentId = rawComponents.length;
      const queue = [startCellId];
      const cellIds = [];
      componentByCell[startCellId] = componentId;
      for (let cursor = 0; cursor < queue.length; cursor++) {
        const cellId = queue[cursor];
        cellIds.push(cellId);
        for (const neighbor of cells.c[cellId] || []) {
          if (componentByCell[neighbor] !== -1 || cellKeys.get(neighbor) !== key) continue;
          componentByCell[neighbor] = componentId;
          queue.push(neighbor);
        }
      }
      const [topologyNodeRef] = key.split("|");
      rawComponents.push({componentId, topologyNodeRef, cellIds});
    }

    const rawNeighbors = Array.from({length: rawComponents.length}, () => new Map());
    for (const cellId of physicalLandCells) {
      const fromComponent = componentByCell[cellId];
      for (const neighbor of cells.c[cellId] || []) {
        if (neighbor <= cellId) continue;
        const toComponent = componentByCell[neighbor];
        if (toComponent < 0 || toComponent === fromComponent) continue;
        rawNeighbors[fromComponent].set(
          toComponent,
          (rawNeighbors[fromComponent].get(toComponent) || 0) + 1
        );
        rawNeighbors[toComponent].set(
          fromComponent,
          (rawNeighbors[toComponent].get(fromComponent) || 0) + 1
        );
      }
    }

    const parent = rawComponents.map(component => component.componentId);
    const mergedCellCount = rawComponents.map(component => component.cellIds.length);
    const findRoot = componentId => {
      let root = componentId;
      while (parent[root] !== root) root = parent[root];
      while (parent[componentId] !== componentId) {
        const next = parent[componentId];
        parent[componentId] = root;
        componentId = next;
      }
      return root;
    };
    for (const component of [...rawComponents].sort((a, b) => a.cellIds.length - b.cellIds.length)) {
      const root = findRoot(component.componentId);
      if (mergedCellCount[root] >= 24) continue;
      const candidates = [...rawNeighbors[component.componentId].entries()]
        .map(([neighbor, border]) => ({root: findRoot(neighbor), border}))
        .filter(candidate => candidate.root !== root)
        .filter(candidate => rawComponents[candidate.root].topologyNodeRef === rawComponents[root].topologyNodeRef)
        .sort(
          (a, b) =>
            b.border - a.border ||
            mergedCellCount[b.root] - mergedCellCount[a.root] ||
            a.root - b.root
        );
      const target = candidates[0]?.root;
      if (target === undefined) continue;
      parent[root] = target;
      mergedCellCount[target] += mergedCellCount[root];
    }

    const mergedComponents = new Map();
    for (const raw of rawComponents) {
      const root = findRoot(raw.componentId);
      const merged = mergedComponents.get(root) || {
        topologyNodeRef: rawComponents[root].topologyNodeRef,
        cellIds: []
      };
      merged.cellIds.push(...raw.cellIds);
      mergedComponents.set(root, merged);
    }
    const components = [...mergedComponents.values()].map((component, componentId) => ({
      componentId,
      ...component
    }));
    for (const component of components) {
      for (const cellId of component.cellIds) componentByCell[cellId] = component.componentId;
    }

    const componentStats = components.map(component => {
      let xSum = 0;
      let ySum = 0;
      let heightSum = 0;
      let minHeight = 100;
      let maxHeight = 0;
      let minX = 1;
      let maxX = 0;
      let minY = 1;
      let maxY = 0;
      let riverCellCount = 0;
      const biomeCounts = new Map();
      const terrainCounts = new Map();
      const elevationCounts = new Map();
      const riverCounts = new Map();
      const featureCounts = new Map();
      for (const cellId of component.cellIds) {
        const [pointX, pointY] = cells.p[cellId];
        const x = pointX / graphWidth;
        const y = pointY / graphHeight;
        const height = cells.h[cellId];
        xSum += x;
        ySum += y;
        heightSum += height;
        minHeight = Math.min(minHeight, height);
        maxHeight = Math.max(maxHeight, height);
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
        const biomeName = biomesData.name[cells.biome[cellId]];
        biomeCounts.set(biomeName, (biomeCounts.get(biomeName) || 0) + 1);
        const terrain = terrainClass(biomeName, height);
        terrainCounts.set(terrain, (terrainCounts.get(terrain) || 0) + 1);
        const elevation = elevationBand(height);
        elevationCounts.set(elevation, (elevationCounts.get(elevation) || 0) + 1);
        featureCounts.set(cells.f[cellId], (featureCounts.get(cells.f[cellId]) || 0) + 1);
        if (cells.r[cellId]) {
          riverCellCount++;
          riverCounts.set(cells.r[cellId], (riverCounts.get(cells.r[cellId]) || 0) + 1);
        }
      }
      const count = component.cellIds.length;
      const sortedBiomes = [...biomeCounts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
      const dominantTerrain = [...terrainCounts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0]?.[0] ?? "mixed_ground";
      const dominantElevation = [...elevationCounts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0]?.[0] ?? "lowland";
      const sortedRivers = [...riverCounts.entries()].sort((a, b) => b[1] - a[1] || a[0] - b[0]);
      const dominantFeature = [...featureCounts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;
      return {
        ...component,
        terrain: dominantTerrain,
        elevation: dominantElevation,
        cellCount: count,
        centroid: {x: round(xSum / count), y: round(ySum / count)},
        bounds: {minX: round(minX), minY: round(minY), maxX: round(maxX), maxY: round(maxY)},
        height: {min: minHeight, mean: round(heightSum / count, 3), max: maxHeight},
        biomeNames: sortedBiomes.map(([name]) => name),
        dominantBiome: sortedBiomes[0]?.[0] ?? "Unknown",
        riverCellCount,
        majorRiverIds: sortedRivers.slice(0, 8).map(([riverId]) => riverId),
        featureId: dominantFeature
      };
    });

    componentStats.sort(
      (a, b) =>
        a.topologyNodeRef.localeCompare(b.topologyNodeRef) ||
        a.terrain.localeCompare(b.terrain) ||
        a.elevation.localeCompare(b.elevation) ||
        a.centroid.y - b.centroid.y ||
        a.centroid.x - b.centroid.x
    );
    const componentAreaId = new Map();
    const nameCounts = new Map();
    const terrainLabels = {
      snow_and_ice: "雪冰地",
      wetland: "濕地",
      forest: "森林",
      grassland: "草地",
      tundra: "凍原",
      open_bare_ground: "裸地",
      mixed_ground: "混合地表"
    };
    const elevationLabels = {alpine: "高山", highland: "高地", upland: "丘地", lowland: "低地"};
    const anchorById = new Map(landAnchors.map(anchor => [anchor.id, anchor]));
    const areas = componentStats.map((component, index) => {
      const id = `wt-physical-${String(index + 1).padStart(4, "0")}`;
      componentAreaId.set(component.componentId, id);
      const anchor = anchorById.get(component.topologyNodeRef);
      const baseName = `${anchor.label}・${elevationLabels[component.elevation]}${terrainLabels[component.terrain]}`;
      const ordinal = (nameCounts.get(baseName) || 0) + 1;
      nameCounts.set(baseName, ordinal);
      return {
        id,
        worldId: "white_tower_peninsula",
        topologyNodeRef: component.topologyNodeRef,
        parentPlaceId: `topology:${component.topologyNodeRef}`,
        name: `${baseName} ${String(ordinal).padStart(2, "0")}`,
        kind: "physical_area_candidate",
        scale: "terrain_area",
        canon: false,
        status: "material",
        traversable: true,
        sourceKind: "azgaar_physical",
        terrainClass: component.terrain,
        elevationBand: component.elevation,
        biomeNames: component.biomeNames,
        featureId: component.featureId,
        sourceCellCount: component.cellCount,
        centroid: component.centroid,
        bounds: component.bounds,
        height: component.height,
        riverCellCount: component.riverCellCount,
        majorRiverIds: component.majorRiverIds,
        description: `${anchor.label}附近的${elevationLabels[component.elevation]}${terrainLabels[component.terrain]}；這是物理地理聚合候選，不是正式地名。`
      };
    });

    const sharedBorders = new Map();
    for (const cellId of physicalLandCells) {
      const fromComponent = componentByCell[cellId];
      for (const neighbor of cells.c[cellId] || []) {
        if (neighbor <= cellId) continue;
        const toComponent = componentByCell[neighbor];
        if (toComponent < 0 || toComponent === fromComponent) continue;
        const low = Math.min(fromComponent, toComponent);
        const high = Math.max(fromComponent, toComponent);
        const key = `${low}:${high}`;
        sharedBorders.set(key, (sharedBorders.get(key) || 0) + 1);
      }
    }
    const areaById = new Map(areas.map(area => [area.id, area]));
    const directionHint = (from, to) => {
      const dx = to.centroid.x - from.centroid.x;
      const dy = to.centroid.y - from.centroid.y;
      return Math.abs(dx) >= Math.abs(dy) ? (dx >= 0 ? "東" : "西") : dy >= 0 ? "南" : "北";
    };
    const terrainPenalty = area => {
      if (area.terrainClass === "wetland") return 8;
      if (area.elevationBand === "alpine") return 10;
      if (area.elevationBand === "highland") return 6;
      if (area.terrainClass === "forest") return 4;
      return 1;
    };
    const routeCandidates = [];
    for (const [key, sharedBorderCells] of [...sharedBorders.entries()].sort()) {
      const [leftComponent, rightComponent] = key.split(":").map(Number);
      const leftId = componentAreaId.get(leftComponent);
      const rightId = componentAreaId.get(rightComponent);
      const left = areaById.get(leftId);
      const right = areaById.get(rightId);
      const distance = Math.hypot(left.centroid.x - right.centroid.x, left.centroid.y - right.centroid.y);
      const relativeCost = Math.max(1, Math.round(distance * 100 + (terrainPenalty(left) + terrainPenalty(right)) / 2));
      for (const [from, to] of [[left, right], [right, left]]) {
        routeCandidates.push({
          id: `wt-route-${String(routeCandidates.length + 1).padStart(5, "0")}`,
          fromPlaceId: from.id,
          direction: directionHint(from, to),
          toPlaceId: to.id,
          mode: from.terrainClass === "wetland" || to.terrainClass === "wetland" ? "difficult_ground" : "walk",
          relativeCost,
          sharedBorderCells,
          canon: false,
          status: "candidate",
          sourceKind: "azgaar_physical"
        });
      }
    }

    const anchorAreaMappings = anchorCellMappings.map(mapping => ({
      ...mapping,
      areaRef:
        mapping.expectedSurface === "land" && componentByCell[mapping.cellId] >= 0
          ? componentAreaId.get(componentByCell[mapping.cellId])
          : null
    }));
    const islandArea = anchorAreaMappings.find(mapping => mapping.topologyNodeRef === "wt-w-windship-island")?.areaRef;
    const capeArea = anchorAreaMappings.find(mapping => mapping.topologyNodeRef === "wt-w-windship-cape")?.areaRef;
    if (islandArea && capeArea) {
      for (const [fromPlaceId, direction, toPlaceId] of [
        [islandArea, "東", capeArea],
        [capeArea, "西", islandArea]
      ]) {
        routeCandidates.push({
          id: `wt-route-${String(routeCandidates.length + 1).padStart(5, "0")}`,
          fromPlaceId,
          direction,
          toPlaceId,
          mode: "ferry",
          relativeCost: 8,
          sharedBorderCells: 0,
          viaTopologyNodeRef: "wt-w-natural-harbor-channel",
          canon: false,
          status: "candidate",
          sourceKind: "azgaar_physical"
        });
      }
    }

    const finiteOrNull = value => (Number.isFinite(value) ? round(value, 3) : null);
    const rivers = pack.rivers
      .filter(river => river?.i)
      .map(river => ({
        sourceRiverId: river.i,
        parentRiverId: river.parent || 0,
        basinId: river.basin || river.i,
        sourceCellId: Number.isInteger(river.source) ? river.source : null,
        mouthCellId: Number.isInteger(river.mouth) ? river.mouth : null,
        length: finiteOrNull(river.length),
        discharge: finiteOrNull(river.discharge),
        width: finiteOrNull(river.width)
      }))
      .sort((a, b) => a.sourceRiverId - b.sourceRiverId);

    const normalizedCellPoint = cellId => {
      const [pointX, pointY] = cells.p[cellId];
      return {x: round(pointX / graphWidth), y: round(pointY / graphHeight)};
    };
    const pointDistance = (left, right) => Math.hypot(left.x - right.x, left.y - right.y);
    const downstreamOptions = new Map();
    for (const river of pack.rivers) {
      for (let index = 0; index < river.cells.length - 1; index++) {
        const from = river.cells[index];
        const to = river.cells[index + 1];
        if (from < 0 || to < 0) continue;
        const options = downstreamOptions.get(from) || [];
        options.push({to, riverId: river.i});
        downstreamOptions.set(from, options);
      }
    }
    const downstream = new Map();
    for (const [from, options] of downstreamOptions) {
      options.sort(
        (left, right) =>
          Number(right.riverId === cells.r[from]) - Number(left.riverId === cells.r[from]) ||
          cells.h[left.to] - cells.h[right.to] ||
          cells.fl[right.to] - cells.fl[left.to] ||
          left.to - right.to
      );
      downstream.set(from, options[0].to);
    }
    const traceDownstream = startCellId => {
      const path = [];
      const seen = new Set();
      let cellId = startCellId;
      while (cellId >= 0 && !seen.has(cellId) && path.length < cells.i.length) {
        path.push(cellId);
        seen.add(cellId);
        if (!downstream.has(cellId)) break;
        cellId = downstream.get(cellId);
      }
      return path;
    };
    const closestPathPoint = (path, expected, startIndex = 0) => {
      let best = {distance: Infinity, index: startIndex};
      for (let index = startIndex; index < path.length; index++) {
        const distance = pointDistance(normalizedCellPoint(path[index]), expected);
        if (distance < best.distance) best = {distance, index};
      }
      return best;
    };
    const scoreHeadwater = (river, checkpoints) => {
      const path = traceDownstream(river.source);
      let cursor = 0;
      let score = pointDistance(normalizedCellPoint(river.source), checkpoints[0]) * 4;
      const checkpointDistances = [];
      for (const checkpoint of checkpoints) {
        const closest = closestPathPoint(path, checkpoint, cursor);
        checkpointDistances.push(round(closest.distance));
        score += closest.distance;
        cursor = closest.index;
      }
      if (path.length < 20) score += 2;
      return {river, path, score, checkpointDistances};
    };
    const headwaterSpecs = [
      {
        topologyNodeRef: "wt-c-north-source-river",
        structuralLabel: "中部三河北來源",
        role: "north_headwater",
        checkpoints: [
          {x: 0.49, y: 0.17},
          {x: 0.47, y: 0.31},
          {x: 0.455, y: 0.445},
          {x: 0.465, y: 0.47},
          {x: 0.478, y: 0.5},
          {x: 0.49, y: 0.6},
          {x: 0.53, y: 0.76},
          {x: 0.5, y: 0.95}
        ]
      },
      {
        topologyNodeRef: "wt-c-west-source-river",
        structuralLabel: "中部三河西來源",
        role: "west_headwater",
        checkpoints: [
          {x: 0.31, y: 0.29},
          {x: 0.37, y: 0.39},
          {x: 0.44, y: 0.445},
          {x: 0.45, y: 0.475},
          {x: 0.478, y: 0.5},
          {x: 0.49, y: 0.6},
          {x: 0.53, y: 0.76},
          {x: 0.5, y: 0.95}
        ]
      },
      {
        topologyNodeRef: "wt-c-east-source-river",
        structuralLabel: "中部三河東來源",
        role: "east_headwater",
        checkpoints: [
          {x: 0.68, y: 0.27},
          {x: 0.58, y: 0.38},
          {x: 0.5, y: 0.44},
          {x: 0.495, y: 0.47},
          {x: 0.478, y: 0.5},
          {x: 0.49, y: 0.6},
          {x: 0.53, y: 0.76},
          {x: 0.5, y: 0.95}
        ]
      }
    ];
    const selectedHeadwaters = headwaterSpecs.map(spec => {
      const candidates = pack.rivers
        .filter(river => pointDistance(normalizedCellPoint(river.source), spec.checkpoints[0]) <= 0.14)
        .map(river => scoreHeadwater(river, spec.checkpoints))
        .sort((left, right) => left.score - right.score || right.river.discharge - left.river.discharge);
      if (!candidates.length) throw new Error(`No generated headwater candidate for ${spec.topologyNodeRef}`);
      return {...spec, ...candidates[0]};
    });
    const otherPathSets = selectedHeadwaters.slice(1).map(selection => new Set(selection.path));
    const confluenceIndex = selectedHeadwaters[0].path.findIndex(cellId =>
      otherPathSets.every(pathSet => pathSet.has(cellId))
    );
    if (confluenceIndex < 0) throw new Error("Generated headwaters do not share a lower channel");
    const confluenceCellId = selectedHeadwaters[0].path[confluenceIndex];
    const confluencePoint = normalizedCellPoint(confluenceCellId);
    const simplifyPath = path => {
      const step = Math.max(1, Math.ceil(path.length / 80));
      return path
        .filter((_, index) => index === 0 || index === path.length - 1 || index % step === 0)
        .map(normalizedCellPoint);
    };
    const riverIdsForPath = path => [...new Set(path.map(cellId => Number(cells.r[cellId])).filter(Boolean))];
    const branchWatercourses = selectedHeadwaters.map(selection => {
      const branchEnd = selection.path.indexOf(confluenceCellId);
      const path = selection.path.slice(0, branchEnd + 1);
      return {
        topologyNodeRef: selection.topologyNodeRef,
        structuralLabel: selection.structuralLabel,
        role: selection.role,
        status: "working_proposal",
        canon: false,
        startRiverId: selection.river.i,
        basinId: selection.river.basin,
        startCellId: path[0],
        endCellId: confluenceCellId,
        sourceFitDistance: round(pointDistance(normalizedCellPoint(path[0]), selection.checkpoints[0])),
        confluenceFitDistance: round(pointDistance(confluencePoint, {x: 0.46, y: 0.45})),
        generatedRiverIds: riverIdsForPath(path),
        sourceCellCount: path.length,
        startPoint: normalizedCellPoint(path[0]),
        endPoint: confluencePoint,
        path: simplifyPath(path)
      };
    });
    const lowerPath = selectedHeadwaters[0].path.slice(confluenceIndex);
    const blackMarshFit = closestPathPoint(lowerPath, {x: 0.54, y: 0.74});
    const lowerRiverId = Number(cells.r[confluenceCellId]);
    const lowerRiver = pack.rivers.find(river => river.i === lowerRiverId);
    const lowerWatercourse = {
      topologyNodeRef: "wt-c-central-lower-river",
      structuralLabel: "中部下游大河",
      role: "shared_lower_river",
      status: "working_proposal",
      canon: false,
      startRiverId: lowerRiverId,
      basinId: lowerRiver?.basin || selectedHeadwaters[0].river.basin,
      startCellId: confluenceCellId,
      endCellId: lowerPath.at(-1),
      sourceFitDistance: round(pointDistance(confluencePoint, {x: 0.46, y: 0.45})),
      blackMarshFitDistance: round(blackMarshFit.distance),
      generatedRiverIds: riverIdsForPath(lowerPath),
      sourceCellCount: lowerPath.length,
      startPoint: confluencePoint,
      endPoint: normalizedCellPoint(lowerPath.at(-1)),
      path: simplifyPath(lowerPath)
    };
    const watercourses = [...branchWatercourses, lowerWatercourse];
    const majorHydrology = {
      status: "working_proposal",
      canon: false,
      confluenceCellId,
      confluencePoint,
      watercourses,
      invariants: {
        distinctHeadwaters: new Set(branchWatercourses.map(course => course.startRiverId)).size === 3,
        sharedBasin: new Set(watercourses.map(course => course.basinId)).size === 1,
        confluenceNearWenyao: pointDistance(confluencePoint, {x: 0.46, y: 0.45}) <= 0.075,
        lowerRiverReachesBlackMarsh: blackMarshFit.distance <= 0.055,
        lowerRiverContinuesSouth:
          lowerWatercourse.endPoint.y >= 0.94 && Math.abs(lowerWatercourse.endPoint.x - 0.5) <= 0.12
      },
      selectionDiagnostics: selectedHeadwaters.map(selection => ({
        topologyNodeRef: selection.topologyNodeRef,
        score: round(selection.score),
        checkpointDistances: selection.checkpointDistances
      }))
    };

    const hydrologyColors = ["#0067c5", "#d66500", "#7b2cbf", "#00875a"];
    const hydrologyGroup = document.createElementNS("http://www.w3.org/2000/svg", "g");
    hydrologyGroup.id = "whiteTowerMajorHydrology";
    watercourses.forEach((course, index) => {
      const points = course.path.map(point => `${point.x * graphWidth},${point.y * graphHeight}`).join(" ");
      for (const [stroke, width] of [["#fff8e8", "7"], [hydrologyColors[index], "4"]]) {
        const polyline = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
        polyline.setAttribute("points", points);
        polyline.setAttribute("fill", "none");
        polyline.setAttribute("stroke", stroke);
        polyline.setAttribute("stroke-width", width);
        polyline.setAttribute("stroke-linecap", "round");
        polyline.setAttribute("stroke-linejoin", "round");
        hydrologyGroup.append(polyline);
      }
    });
    document.querySelector("#viewbox").append(hydrologyGroup);
    const usedBiomeIds = [...new Set(physicalLandCells.map(cellId => cells.biome[cellId]))].sort((a, b) => a - b);

    const geography = {
      schemaVersion: 1,
      id: "white_tower_physical_geography_v1",
      name: "白塔半島物理地理橋接資料 v1",
      status: "working_proposal",
      canon: false,
      runtimeAuthority: false,
      playerFacingMap: false,
      source: {
        project: "Azgaar/Fantasy-Map-Generator",
        commit: "73e43d2b44b9b85b42c5394a0476d90bb22d688e",
        seed,
        heightmapProfile: "whiteTowerConstrained",
        canonAuthority: "data/world/white_tower_topology_v1.json"
      },
      coordinateSpace: {
        kind: "normalized_authoring",
        width: graphWidth,
        height: graphHeight,
        physicalAggregateSize,
        formalScaleDecided: false,
        coordinateSystemDecided: false
      },
      metrics: {
        gridCells: grid.cells.i.length,
        packedCells: cells.i.length,
        generatedLandCells: landCells.length,
        aggregatedLandCells: physicalLandCells.length,
        omittedTinyIsletCells: landCells.length - physicalLandCells.length,
        physicalAreas: areas.length,
        routeCandidates: routeCandidates.length,
        rivers: rivers.length,
        lakes: pack.features.filter(feature => feature?.type === "lake").length
      },
      landFeatures,
      biomes: usedBiomeIds.map(id => ({id, name: biomesData.name[id]})),
      anchorMappings: anchorAreaMappings,
      areas,
      routeCandidates,
      rivers,
      majorHydrology,
      invariants: {
        oneMainlandAndWindshipIsland: true,
        windshipChannelRemainsWater: true,
        whiteTowerHeadsGreenRidge: true,
        greenRidgeMeetsEastSea: true,
        threeSourceValleysMeetNearWenyao: true,
        lowerRiverReachesBlackMarsh: true,
        blackMarshIsWetlandNotLake: true,
        centralForestIsNotBlackMarsh: true
      },
      sourceDispositions: {
        kept: ["cell adjacency", "height", "biome", "river hierarchy", "land features", "deterministic provenance"],
        candidateOnly: [
          "aggregated physical areas",
          "direction hints",
          "relative travel costs",
          "generated names",
          "burgs",
          "routes",
          "states",
          "provinces",
          "cultures",
          "religions",
          "politics",
          "markers",
          "zones",
          "notes",
          "goods",
          "markets",
          "deals"
        ],
        rejected: ["renderer as player UI"]
      }
    };
    return {summary, geography};
  });

  const {summary, geography} = exported;
  summary.consoleErrors = consoleErrors;
  summary.consoleWarnings = consoleWarnings;
  summary.rejectedSurfaceErrors = rejectedSurfaceErrors;
  summary.pageErrors = pageErrors;
  summary.source.commit = "73e43d2b44b9b85b42c5394a0476d90bb22d688e";

  if (evidenceDir) {
    await page.screenshot({path: outputPng, fullPage: true});
    await page.evaluate(() => {
      toggleHeight();
      toggleBiomes();
    });
    await page.screenshot({path: outputBiomePng, fullPage: true});
  }
  geography.generationEvidence = {
    consoleErrors,
    consoleWarnings,
    rejectedSurfaceErrors,
    pageErrors
  };
  if (packageFiles) {
    geography.generationEvidence.nativePackageManifest =
      "data/world/white_tower_azgaar_v1/manifest.json";
  }
  if (outputSummary) {
    await fs.writeFile(outputSummary, `${JSON.stringify(summary, null, 2)}\n`);
  }
  await fs.writeFile(outputGeography, `${JSON.stringify(geography, null, 2)}\n`);
  const physicalGeographyReceipt = await fileReceipt(outputGeography);
  const nativePackage = await exportNativePackage(page, summary, physicalGeographyReceipt);
  console.log(
    JSON.stringify(
      {
        summary,
        geography: geography.metrics,
        nativePackage: nativePackage ? packageFiles.manifest : null,
        outputPng,
        outputBiomePng,
        outputSummary,
        outputGeography
      },
      null,
      2
    )
  );
} finally {
  await context.close();
  await browser.close();
}
