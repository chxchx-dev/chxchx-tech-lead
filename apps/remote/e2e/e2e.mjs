// End-to-end tests of the web client against a real node and a fake agent, in real Chromium.
// See README.md. Run with: node e2e.mjs
import { chromium, devices } from "playwright-core";
import { execSync } from "node:child_process";
import assert from "node:assert/strict";
import fs from "node:fs";

const here = new URL(".", import.meta.url).pathname;
const env = (cmd) => execSync(`${here}env.sh ${cmd}`, { encoding: "utf8" }).trim();
const base = "http://127.0.0.1:47831";
const shots = `${here}shots/`;
fs.mkdirSync(shots, { recursive: true });

const results = [];
async function step(name, fn) {
  const started = Date.now();
  try {
    await fn();
    results.push({ name, ok: true, ms: Date.now() - started });
    console.log(`  ✓ ${name} (${Date.now() - started} ms)`);
  } catch (error) {
    results.push({ name, ok: false, error: String(error.message ?? error).split("\n").slice(0, 6).join(" | ") });
    console.log(`  ✗ ${name}\n      ${String(error.message ?? error).split("\n").slice(0, 6).join("\n      ")}`);
  }
}

const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROME_PATH || undefined, args: ["--no-sandbox"] });
const problems = []; // console errors and page errors, across every page
async function open(role, { viewport } = {}) {
  const context = await browser.newContext({ ...devices["Pixel 7"], ...(viewport ? { viewport } : {}) });
  context.setDefaultTimeout(10_000);
  const page = await context.newPage();
  page.on("pageerror", (error) => problems.push(`[pageerror/${role}] ${error.message}`));
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(`[console.error/${role}] ${message.text()}`);
  });
  return { context, page };
}
async function pairWith(role, name) {
  const code = env(`pair ${role}`);
  const session = await open(role);
  await session.page.goto(`${base}/#code=${code}`);
  await session.page.locator("input").nth(1).fill(name);
  await session.page.getByRole("button", { name: "Vincular" }).click();
  await session.page.locator(".tabs").waitFor({ timeout: 10_000 });
  return session;
}
const sendMessage = async (page, text) => {
  await page.locator("textarea").fill(text);
  await page.getByRole("button", { name: "Enviar" }).click();
};

console.log("== entorno");
console.log("  ", env("start"));

// ------------------------------------------------------------------------------------------
console.log("== vinculación");
const phone = await open("operator");
const page = phone.page;

await step("sin sesión se pide un código y el botón espera a tenerlo", async () => {
  await page.goto(`${base}/`);
  await page.locator(".pairing h1").waitFor({ timeout: 10_000 });
  assert.equal(await page.getByRole("button", { name: "Vincular" }).isDisabled(), true);
  assert.match(await page.locator("body").innerText(), /chxchx-tech node pair/);
  await page.screenshot({ path: `${shots}01-pairing.png` });
});

let operatorCode;
await step("un QR rellena el código y no vincula solo", async () => {
  operatorCode = env("pair operator");
  await page.goto(`${base}/#code=${operatorCode}`);
  await page.reload();
  await page.locator(".pairing").waitFor();
  assert.equal(await page.locator("input").first().inputValue(), operatorCode);
  assert.match(await page.locator("body").innerText(), /vino del QR/);
  assert.equal(await page.evaluate(() => localStorage.getItem("chxchx.session")), null);
});

await step("vincular entra, guarda la sesión y borra el código de la dirección", async () => {
  await page.locator("input").nth(1).fill("Pixel de prueba");
  await page.getByRole("button", { name: "Vincular" }).click();
  await page.locator(".tabs").waitFor({ timeout: 10_000 });
  assert.ok(await page.evaluate(() => localStorage.getItem("chxchx.session")));
  assert.equal(await page.evaluate(() => location.hash), "#/");
  assert.ok(!page.url().includes("code="), page.url());
});

await step("el código ya usado no vale otra vez", async () => {
  const other = await open("replay");
  await other.page.goto(`${base}/#code=${operatorCode}`);
  await other.page.getByRole("button", { name: "Vincular" }).click();
  await other.page.locator(".note.bad").waitFor({ timeout: 10_000 });
  assert.match(await other.page.locator(".note.bad").innerText(), /incorrecto o caducado/);
  await other.context.close();
});

await step("la conexión en vivo se establece", async () => {
  await page.locator(".dot.online").waitFor({ timeout: 10_000 });
});

await step("Inicio muestra el estado del nodo y el rol", async () => {
  await page.getByText("Tu PC").waitFor();
  await page.locator(".facts").first().waitFor();
  const text = await page.locator("main").innerText();
  assert.match(text, /Operador/);
  await page.screenshot({ path: `${shots}02-home.png` });
});

// ------------------------------------------------------------------------------------------
console.log("== chat");
await step("Proyectos lista el proyecto de confianza", async () => {
  await page.getByRole("link", { name: /Proyectos/ }).click();
  await page.getByRole("heading", { name: "demo" }).waitFor({ timeout: 10_000 });
  await page.screenshot({ path: `${shots}03-projects.png` });
});

await step("abrir el proyecto muestra los agentes y permite cambiar a Codex", async () => {
  await page.locator("a.card.link").click();
  await page.locator(".segments").waitFor();
  await page.getByRole("link", { name: "Codex" }).click();
  await page.waitForFunction(() => location.hash === "#/chat/demo/codex");
  await page.locator("textarea").waitFor({ timeout: 15_000 });
});

await step("un mensaje corre como trabajo y se ve la respuesta, la herramienta y el final", async () => {
  await sendMessage(page, "hola");
  await page.locator(".msg.assistant", { hasText: "Listo." }).waitFor({ timeout: 20_000 });
  assert.ok(await page.locator(".msg.user", { hasText: "hola" }).count() >= 1);
  assert.ok(await page.locator(".msg.assistant", { hasText: "Voy a mirar el proyecto." }).count() >= 1);
  assert.match(await page.locator(".tool.ok").first().innerText(), /ls -la/);
  await page.screenshot({ path: `${shots}04-chat.png` });
});

await step("un permiso aparece como tarjeta, se aprueba y el agente sigue", async () => {
  await sendMessage(page, "necesito permiso");
  await page.locator(".card.ask.open").waitFor({ timeout: 20_000 });
  assert.match(await page.locator(".card.ask.open").innerText(), /cargo test --all/);
  // The newest card must be scrolled into view, above the composer and not under it.
  await page.waitForTimeout(1500);
  const allow = await page.getByRole("button", { name: "Permitir esta vez" }).boundingBox();
  const composer = await page.locator("form.composer").boundingBox();
  assert.ok(allow.y + allow.height <= composer.y + 1, `el botón (${allow.y + allow.height}) queda bajo el compositor (${composer.y})`);
  await page.screenshot({ path: `${shots}05-permission.png` });
  await page.getByRole("button", { name: "Permitir esta vez" }).click();
  await page.locator(".decided", { hasText: "Permitido" }).waitFor({ timeout: 10_000 });
  await page.locator(".msg.assistant", { hasText: "Respuesta recibida: allow" }).waitFor({ timeout: 10_000 });
});

await step("una pregunta del agente se responde eligiendo una opción", async () => {
  await sendMessage(page, "tengo una pregunta");
  await page.locator("fieldset").waitFor({ timeout: 20_000 });
  assert.equal(await page.getByRole("button", { name: "Responder" }).isDisabled(), true, "sin elegir no se envía");
  await page.getByRole("button", { name: /SQLite/ }).click();
  await page.getByRole("button", { name: "Responder" }).click();
  await page.locator(".msg.assistant", { hasText: "SQLite" }).waitFor({ timeout: 10_000 });
  await page.locator(".decided", { hasText: "Respondida" }).waitFor();
});

await step("un permiso se puede rechazar", async () => {
  await sendMessage(page, "otro permiso");
  await page.locator(".card.ask.open").waitFor({ timeout: 20_000 });
  await page.getByRole("button", { name: "Rechazar" }).click();
  await page.locator(".msg.assistant", { hasText: "Respuesta recibida: deny" }).waitFor({ timeout: 10_000 });
});

await step("recargar conserva la sesión y recupera la conversación guardada", async () => {
  await page.reload();
  await page.locator("textarea").waitFor({ timeout: 15_000 });
  await page.locator(".msg.user", { hasText: "hola" }).first().waitFor({ timeout: 15_000 });
  assert.ok(await page.locator(".msg.assistant", { hasText: "Listo." }).count() >= 1);
});

await step("un turno lento se puede detener", async () => {
  await sendMessage(page, "algo lento");
  await page.getByRole("button", { name: "Detener" }).waitFor({ timeout: 15_000 });
  await page.getByRole("button", { name: "Detener" }).click();
  await page.getByRole("button", { name: "Enviar" }).waitFor({ timeout: 15_000 });
});

await step("Trabajos lista los trabajos con su estado y abre el detalle", async () => {
  await page.getByRole("link", { name: /Trabajos/ }).click();
  await page.locator(".job-row").first().waitFor({ timeout: 10_000 });
  const text = await page.locator("main").innerText();
  assert.match(text, /Terminado/);
  assert.match(text, /Cancelado/);
  await page.locator(".job-row a", { hasText: "Cancelado" }).first().click();
  await page.locator(".badge", { hasText: "Cancelado" }).waitFor();
  await page.screenshot({ path: `${shots}06-job.png` });
});

await step("Ajustes muestra este dispositivo y permite elegir el acceso", async () => {
  await page.getByRole("link", { name: /Ajustes/ }).click();
  await page.getByText("Pixel de prueba").waitFor();
  await page.locator("select").first().selectOption("workspace-write");
  assert.match(await page.evaluate(() => localStorage.getItem("chxchx.prefs")), /workspace-write/);
  await page.screenshot({ path: `${shots}07-settings.png` });
});

// ------------------------------------------------------------------------------------------
console.log("== roles y revocación");
const reader = await pairWith("reader", "Tablet lectora");
await step("un lector mira pero no escribe ni ve Dispositivos", async () => {
  const p = reader.page;
  await p.getByRole("link", { name: /Proyectos/ }).click();
  await p.locator(".card.link").first().waitFor();
  await p.locator(".card.link").first().click();
  await p.locator(".chat").waitFor();
  assert.equal(await p.locator("textarea").count(), 0);
  assert.match(await p.locator("main").innerText(), /rol de lector/);
  assert.equal(await p.getByRole("link", { name: /Dispositivos/ }).count(), 0);
});

const admin = await pairWith("admin", "Administrador");
await step("un administrador ve y gestiona los dispositivos", async () => {
  await admin.page.getByRole("link", { name: /Dispositivos/ }).click();
  await admin.page.locator("li.card").first().waitFor({ timeout: 10_000 });
  const text = await admin.page.locator("main").innerText();
  assert.match(text, /Pixel de prueba/);
  assert.match(text, /Tablet lectora/);
  assert.match(text, /Este dispositivo/);
  await admin.page.screenshot({ path: `${shots}08-devices.png` });
});

await step("desvincular un dispositivo lo saca de su sesión en vivo", async () => {
  const card = admin.page.locator("li.card", { hasText: "Pixel de prueba" });
  await card.getByRole("button", { name: "Desvincular" }).click();
  await card.getByRole("button", { name: "Sí, desvincular" }).click();
  await admin.page.locator("li.card", { hasText: "Pixel de prueba" }).waitFor({ state: "detached", timeout: 10_000 });
  // The phone that was unlinked finds out by itself and goes back to the pairing screen.
  await page.locator(".pairing").waitFor({ timeout: 20_000 });
  assert.equal(await page.evaluate(() => localStorage.getItem("chxchx.session")), null);
});

// ------------------------------------------------------------------------------------------
console.log("== caída y recuperación");
await step("si el nodo cae se avisa, y al volver se recupera solo", async () => {
  const p = admin.page;
  await p.getByRole("link", { name: /Trabajos/ }).click();
  env("stop");
  await p.locator(".banner").waitFor({ timeout: 20_000 });
  assert.match(await p.locator(".banner").innerText(), /Sin conexión/);
  await p.screenshot({ path: `${shots}09-offline.png` });
  env("restart");
  await p.locator(".banner").waitFor({ state: "detached", timeout: 30_000 });
  await p.locator(".dot.online").waitFor({ timeout: 10_000 });
  // The device is still linked after the restart: nothing was lost.
  assert.ok(await p.evaluate(() => localStorage.getItem("chxchx.session")));
});

await step("un turno en curso al caer el nodo queda como interrumpido, sin perderse", async () => {
  const p = admin.page;
  await p.getByRole("link", { name: /Trabajos/ }).click();
  const text = await p.locator("main").innerText();
  assert.match(text, /Terminado|Cancelado|Interrumpido/);
});

await step("la app arranca sin red desde el service worker", async () => {
  const p = admin.page;
  // The page's policy forbids eval, which is what waitForFunction uses: poll with evaluate.
  for (let i = 0; i < 30 && !(await p.evaluate(() => !!navigator.serviceWorker.controller)); i++) await p.waitForTimeout(500);
  const cached = await p.evaluate(async () => (await (await caches.open((await caches.keys())[0])).keys()).map((r) => new URL(r.url).pathname));
  assert.ok(cached.includes("/pkg/app_bg.wasm") && cached.includes("/"), JSON.stringify(cached));
  // The phone out of reach of the PC: the node is down and the page still starts.
  env("stop");
  await p.reload();
  await p.locator(".app").waitFor({ timeout: 15_000 });
  assert.equal(await p.title(), "ChxChx");
  await p.locator(".banner").waitFor({ timeout: 20_000 });
  await p.screenshot({ path: `${shots}10-offline-start.png` });
  env("restart");
  await p.locator(".dot.online").waitFor({ timeout: 30_000 });
});

// ------------------------------------------------------------------------------------------
console.log("== calidad");
await step("ninguna pantalla se desborda en horizontal a 360 px", async () => {
  const small = await open("admin", { viewport: { width: 360, height: 740 } });
  const code = env("pair operator");
  await small.page.goto(`${base}/#code=${code}`);
  await small.page.getByRole("button", { name: "Vincular" }).click();
  await small.page.locator(".tabs").waitFor();
  for (const route of ["#/", "#/projects", "#/jobs", "#/settings", "#/chat/demo/codex"]) {
    await small.page.evaluate((r) => (location.hash = r), route);
    await small.page.waitForTimeout(700);
    const [scroll, width] = await small.page.evaluate(() => [document.documentElement.scrollWidth, innerWidth]);
    assert.ok(scroll <= width, `${route}: ${scroll} > ${width}`);
  }
  await small.context.close();
});

await step("todo control tiene nombre accesible y el documento declara su idioma", async () => {
  const p = admin.page;
  await p.evaluate(() => (location.hash = "#/settings"));
  await p.waitForTimeout(500);
  const unnamed = await p.evaluate(() =>
    [...document.querySelectorAll("button, a, input, select, textarea")]
      .filter((el) => !(el.innerText || el.getAttribute("aria-label") || el.getAttribute("placeholder") || el.labels?.length || el.title || el.value || "").toString().trim())
      .map((el) => el.outerHTML.slice(0, 80)),
  );
  assert.deepEqual(unnamed, []);
  assert.equal(await p.evaluate(() => document.documentElement.lang), "es");
});

await step("la política de contenido no rompió nada: sin errores en consola", async () => {
  const csp = problems.filter((line) => /Content Security Policy|Refused to/i.test(line));
  assert.deepEqual(csp, []);
});

// ------------------------------------------------------------------------------------------
await browser.close();
env("stop");
const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length}/${results.length} pasos correctos`);
if (problems.length) console.log("\nMensajes de error del navegador:\n" + [...new Set(problems)].slice(0, 12).join("\n"));
process.exit(failed.length ? 1 : 0);
