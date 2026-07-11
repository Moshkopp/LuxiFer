// Gemeinsame WebGL-Render-Schicht (ADR 0008): EINE Zeichenschicht für alle
// Ansichten (Design, Preview, Laser). Zeichnet Liniensegmente mit Per-Segment-
// Farbe in EINEM Draw-Call — genau das, was CPU-Canvas nicht schafft (Messung
// ADR 0008: WebGL < 1 ms bei 1 Mio Segmenten inkl. Farbverlauf).
//
// Bewusst rohes WebGL, dünn gekapselt (keine Lib, ADR 0008 §1). 1-px-Linien
// (gl.LINES) — scharf auf jeder Zoomstufe, maximal schnell; dickere Linien
// bräuchten Triangle-Expansion und sind vorerst nicht nötig.

import { type Camera, mmToClipMatrix } from "./camera";

/** Ein Batch aus Liniensegmenten in mm mit Per-Vertex-RGBA-Farbe. */
export interface LineBatch {
  /** Flaches Array [x0,y0, x1,y1, …] in mm (2 Punkte je Segment). */
  positions: Float32Array;
  /** Flaches Array [r,g,b,a, …] je VERTEX (also 2 Farben je Segment), 0..1. */
  colors: Float32Array;
}

const VS = `
attribute vec2 a_pos;
attribute vec4 a_col;
uniform mat3 u_mvp;
uniform mat3 u_model;
uniform vec2 u_offset;   // mm-Verschiebung (Live-Move); sonst (0,0)
varying vec4 v_col;
void main() {
  vec3 p = u_mvp * u_model * vec3(a_pos + u_offset, 1.0);
  gl_Position = vec4(p.xy, 0.0, 1.0);
  gl_PointSize = 9.0;
  v_col = a_col;
}`;

const FS = `
precision mediump float;
varying vec4 v_col;
void main() { gl_FragColor = v_col; }`;

// Textur-Programm (ADR 0008 §2): ein Bild-Quad. a_uv sampelt die 1-Kanal-Textur;
// gebrannte Texel (Wert 1) werden hell, nicht-gebrannte transparent.
const TVS = `
attribute vec2 a_pos;
attribute vec2 a_uv;
uniform mat3 u_mvp;
varying vec2 v_uv;
void main() {
  vec3 p = u_mvp * vec3(a_pos, 1.0);
  gl_Position = vec4(p.xy, 0.0, 1.0);
  v_uv = a_uv;
}`;

const IDENTITY_MODEL = new Float32Array([
  1, 0, 0,
  0, 1, 0,
  0, 0, 1,
]);

const TFS = `
precision mediump float;
uniform sampler2D u_tex;
uniform vec3 u_burn;
varying vec2 v_uv;
void main() {
  float on = texture2D(u_tex, v_uv).r; // 1 = gebrannt
  if (on < 0.5) discard;               // nicht gebrannt = transparent
  gl_FragColor = vec4(u_burn, 1.0);
}`;

/**
 * Kapselt einen WebGL-Kontext + das Linien-Programm. Eine Instanz pro Canvas;
 * die Zeichen-Aufrufe (`begin`/`lines`/`points`) laufen pro Frame.
 */
export class GlRenderer {
  private gl: WebGLRenderingContext;
  private prog: WebGLProgram;
  private locPos: number;
  private locCol: number;
  private locMvp: WebGLUniformLocation;
  private locModel: WebGLUniformLocation;
  private locOffset: WebGLUniformLocation;
  // Textur-Programm
  private tprog: WebGLProgram;
  private tPos: number;
  private tUv: number;
  private tMvp: WebGLUniformLocation;
  private tBurn: WebGLUniformLocation;
  private mvp = new Float32Array(9);

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl", { antialias: true, alpha: false, stencil: true });
    if (!gl) throw new Error("WebGL nicht verfügbar");
    this.gl = gl;
    this.prog = linkProgram(gl, VS, FS);
    this.locPos = gl.getAttribLocation(this.prog, "a_pos");
    this.locCol = gl.getAttribLocation(this.prog, "a_col");
    this.locMvp = gl.getUniformLocation(this.prog, "u_mvp")!;
    this.locModel = gl.getUniformLocation(this.prog, "u_model")!;
    this.locOffset = gl.getUniformLocation(this.prog, "u_offset")!;
    this.tprog = linkProgram(gl, TVS, TFS);
    this.tPos = gl.getAttribLocation(this.tprog, "a_pos");
    this.tUv = gl.getAttribLocation(this.tprog, "a_uv");
    this.tMvp = gl.getUniformLocation(this.tprog, "u_mvp")!;
    this.tBurn = gl.getUniformLocation(this.tprog, "u_burn")!;
  }

  /** Frame beginnen: Viewport setzen, Hintergrund löschen, Kamera anwenden. */
  begin(cam: Camera, w: number, h: number, bg: [number, number, number]) {
    const gl = this.gl;
    gl.viewport(0, 0, w, h);
    gl.clearColor(bg[0], bg[1], bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    this.mvp.set(mmToClipMatrix(cam, w, h));
    gl.useProgram(this.prog);
    gl.uniformMatrix3fv(this.locMvp, false, this.mvp);
    gl.uniformMatrix3fv(this.locModel, false, IDENTITY_MODEL);
    gl.uniform2f(this.locOffset, 0, 0); // Frame startet ohne Live-Verschiebung
    // Alpha-Blending für halbtransparente Linien (Grid/Travel) + Texturen.
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  }

  /**
   * Setzt die mm-Verschiebung für nachfolgende `drawBatch`-Aufrufe. Kern des
   * schnellen Live-Move: der Batch liegt EINMAL auf der GPU, pro Frame ändert
   * sich nur dieses Uniform — kein Neu-Bauen/Hochladen der Vertices. Nach dem
   * verschobenen Draw wieder auf (0,0) zurücksetzen, damit statische Batches
   * (Grid, Bett, unbewegte Konturen) an ihrer Stelle bleiben.
   */
  setOffset(dx: number, dy: number) {
    this.gl.useProgram(this.prog);
    this.gl.uniform2f(this.locOffset, dx, dy);
  }

  setModel(model: Float32Array) {
    this.gl.useProgram(this.prog);
    this.gl.uniformMatrix3fv(this.locModel, false, model);
  }

  resetModel() {
    this.setModel(IDENTITY_MODEL);
  }

  /**
   * Lädt einen Batch EINMAL in eigene GPU-Buffer hoch und gibt ein Handle
   * zurück. Bei Pan/Zoom wird nur `drawBatch(handle)` gerufen (kein Neu-Upload)
   * — nur die Kamera-Matrix (Uniform) ändert sich. Das ist der Kern der
   * GPU-Performance: Vertex-Daten werden NICHT pro Frame neu kopiert.
   */
  upload(positions: Float32Array, colors: Float32Array): GlBatch {
    const gl = this.gl;
    const pos = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, pos);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
    const col = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, col);
    gl.bufferData(gl.ARRAY_BUFFER, colors, gl.STATIC_DRAW);
    return { pos, col, count: positions.length / 2 };
  }

  /** Einen hochgeladenen Batch zeichnen (kein Upload). `mode`: LINES | POINTS. */
  drawBatch(b: GlBatch, mode: "lines" | "points") {
    const gl = this.gl;
    if (b.count === 0) return;
    gl.bindBuffer(gl.ARRAY_BUFFER, b.pos);
    gl.enableVertexAttribArray(this.locPos);
    gl.vertexAttribPointer(this.locPos, 2, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, b.col);
    gl.enableVertexAttribArray(this.locCol);
    gl.vertexAttribPointer(this.locCol, 4, gl.FLOAT, false, 0, 0);
    gl.drawArrays(mode === "lines" ? gl.LINES : gl.POINTS, 0, b.count);
  }

  /** Vorhandenen Positionsbuffer mit einer konstanten Farbe zeichnen. */
  drawBatchColor(
    b: GlBatch,
    mode: "lines" | "points",
    color: [number, number, number, number],
  ) {
    const gl = this.gl;
    if (b.count === 0) return;
    gl.bindBuffer(gl.ARRAY_BUFFER, b.pos);
    gl.enableVertexAttribArray(this.locPos);
    gl.vertexAttribPointer(this.locPos, 2, gl.FLOAT, false, 0, 0);
    gl.disableVertexAttribArray(this.locCol);
    gl.vertexAttrib4f(this.locCol, color[0], color[1], color[2], color[3]);
    gl.drawArrays(mode === "lines" ? gl.LINES : gl.POINTS, 0, b.count);
    gl.enableVertexAttribArray(this.locCol);
  }

  uploadStencilFill(
    positions: Float32Array,
    ranges: FillRange[],
    bounds: [number, number, number, number],
    color: [number, number, number, number],
    allSelected: boolean,
    layerId: number,
  ): GlStencilFill {
    const gl = this.gl;
    const pos = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, pos);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
    const [x, y, w, h] = bounds;
    const quad = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
      x, y, x + w, y, x, y + h, x + w, y + h,
    ]), gl.STATIC_DRAW);
    return { pos, quad, count: positions.length / 2, ranges, color, allSelected, layerId };
  }

  /**
   * Even-Odd ohne Ringanalyse: Jeder Kontur-Fan invertiert Bit 0 im Stencil.
   * Überlappende Fan-Dreiecke löschen sich per Parität; anschließend wird nur
   * die gesetzte Layerfläche mit einem Bounding-Quad eingefärbt.
   */
  drawStencilFill(fill: GlStencilFill) {
    this.drawStencilFillParts(fill, new Set<number>(), null, 0, 0);
  }

  drawStencilFillParts(
    fill: GlStencilFill,
    selected: Set<number>,
    selectedModel: Float32Array | null,
    offsetX: number,
    offsetY: number,
  ) {
    const gl = this.gl;
    gl.useProgram(this.prog);
    gl.clearStencil(0);
    gl.clear(gl.STENCIL_BUFFER_BIT);
    gl.enable(gl.STENCIL_TEST);
    gl.stencilMask(0x1);
    gl.stencilFunc(gl.ALWAYS, 1, 0x1);
    gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
    gl.colorMask(false, false, false, false);
    gl.disable(gl.BLEND);

    gl.bindBuffer(gl.ARRAY_BUFFER, fill.pos);
    gl.enableVertexAttribArray(this.locPos);
    gl.vertexAttribPointer(this.locPos, 2, gl.FLOAT, false, 0, 0);
    gl.disableVertexAttribArray(this.locCol);
    gl.vertexAttrib4f(this.locCol, 0, 0, 0, 0);
    const runs: { start: number; count: number; moving: boolean }[] = [];
    for (const range of fill.ranges) {
      const moving = selected.has(range.shapeIdx);
      const prev = runs[runs.length - 1];
      if (prev && prev.moving === moving && prev.start + prev.count === range.start) {
        prev.count += range.count;
      } else {
        runs.push({ start: range.start, count: range.count, moving });
      }
    }
    const drawRanges = () => {
      for (const run of runs) {
        const moving = run.moving;
        if (moving) {
          if (selectedModel) this.setModel(selectedModel); else this.resetModel();
          this.setOffset(offsetX, offsetY);
        } else {
          this.resetModel();
          this.setOffset(0, 0);
        }
        gl.drawArrays(gl.TRIANGLES, run.start, run.count);
      }
    };
    drawRanges();

    gl.colorMask(true, true, true, true);
    gl.stencilMask(0);
    gl.stencilFunc(gl.EQUAL, 1, 0x1);
    gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.vertexAttrib4f(
      this.locCol,
      fill.color[0],
      fill.color[1],
      fill.color[2],
      fill.color[3],
    );
    // Dieselben Dreiecke erneut zeichnen. Der erste Treffer färbt und setzt
    // das Stencilbit auf 0; dadurch wird jedes Even-Odd-Innenpixel genau einmal
    // geblendet, ohne ein transformabhängiges Bounding-Quad.
    gl.stencilOp(gl.KEEP, gl.KEEP, gl.ZERO);
    drawRanges();

    this.resetModel();
    this.setOffset(0, 0);
    gl.disable(gl.STENCIL_TEST);
    gl.stencilMask(0xff);
    gl.enableVertexAttribArray(this.locCol);
  }

  freeStencilFill(fill: GlStencilFill) {
    this.gl.deleteBuffer(fill.pos);
    this.gl.deleteBuffer(fill.quad);
  }

  /** GPU-Buffer eines Batches freigeben (beim Neu-Aufbau der Daten). */
  free(b: GlBatch) {
    this.gl.deleteBuffer(b.pos);
    this.gl.deleteBuffer(b.col);
  }

  /**
   * Lädt eine 1-Kanal-Textur (1 Byte/Texel, 255 = gebrannt) EINMAL hoch samt
   * ihrer mm-Box + Quad/UV-Buffer. `NEAREST`-Sampling → beim Reinzoomen scharfe
   * Pixel (einzelne Rasterzeilen sichtbar, ADR 0008 §2). Wie beim Batch: bei
   * Pan/Zoom wird nur `drawTexture` gerufen, nichts neu hochgeladen.
   */
  uploadTexture(pixels: Uint8Array, w: number, h: number, rect: [number, number, number, number]): GlTexture {
    const gl = this.gl;
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, w, h, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, pixels);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    // Quad aus der mm-Box (zwei Dreiecke) + passende UVs. y↓ im Bild → UV.y umkehren.
    const [x, y, ww, hh] = rect;
    const quad = new Float32Array([
      // pos.x   pos.y     uv.x uv.y
      x,      y,        0, 0,
      x + ww, y,        1, 0,
      x,      y + hh,   0, 1,
      x + ww, y,        1, 0,
      x + ww, y + hh,   1, 1,
      x,      y + hh,   0, 1,
    ]);
    const buf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, quad, gl.STATIC_DRAW);
    return { tex, buf };
  }

  /** Eine hochgeladene Bild-Textur zeichnen (gebrannte Texel in `burn`-Farbe). */
  drawTexture(t: GlTexture, burn: [number, number, number]) {
    const gl = this.gl;
    gl.useProgram(this.tprog);
    gl.uniformMatrix3fv(this.tMvp, false, this.mvp);
    gl.uniform3f(this.tBurn, burn[0], burn[1], burn[2]);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, t.tex);
    gl.bindBuffer(gl.ARRAY_BUFFER, t.buf);
    gl.enableVertexAttribArray(this.tPos);
    gl.vertexAttribPointer(this.tPos, 2, gl.FLOAT, false, 16, 0);
    gl.enableVertexAttribArray(this.tUv);
    gl.vertexAttribPointer(this.tUv, 2, gl.FLOAT, false, 16, 8);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    // Zurück aufs Linien-Programm für nachfolgende lines()/points().
    gl.useProgram(this.prog);
  }

  /** Textur-Ressourcen freigeben. */
  freeTexture(t: GlTexture) {
    this.gl.deleteTexture(t.tex);
    this.gl.deleteBuffer(t.buf);
  }

  /** Ob der Kontext verloren ist (dann muss neu aufgebaut werden). */
  isLost(): boolean {
    return this.gl.isContextLost();
  }
}

/** Handle auf einen hochgeladenen Batch (eigene GPU-Buffer). */
export interface GlBatch {
  pos: WebGLBuffer;
  col: WebGLBuffer;
  count: number;
}

export interface GlStencilFill {
  pos: WebGLBuffer;
  quad: WebGLBuffer;
  count: number;
  ranges: FillRange[];
  color: [number, number, number, number];
  allSelected: boolean;
  layerId: number;
}

export interface FillRange {
  shapeIdx: number;
  start: number;
  count: number;
}

/** Handle auf eine hochgeladene Bild-Textur (Textur + Quad-Buffer). */
export interface GlTexture {
  tex: WebGLTexture;
  buf: WebGLBuffer;
}

function linkProgram(gl: WebGLRenderingContext, vs: string, fs: string): WebGLProgram {
  const compile = (type: number, src: string) => {
    const s = gl.createShader(type)!;
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error("Shader-Fehler: " + gl.getShaderInfoLog(s));
    }
    return s;
  };
  const prog = gl.createProgram()!;
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, vs));
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    throw new Error("Programm-Link-Fehler: " + gl.getProgramInfoLog(prog));
  }
  return prog;
}
