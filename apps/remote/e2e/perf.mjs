import { chromium, devices } from "playwright-core";
import { execSync } from "node:child_process";
const here = new URL(".", import.meta.url).pathname;
const run = (c) => execSync(`${here}env.sh ${c}`, { encoding: "utf8" }).trim();
run("start");
const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROME_PATH || undefined, args: ["--no-sandbox"] });
const profiles = {
  "sin límite": null,
  "4G típico (10 Mbps, 60 ms)": { latency: 60, downloadThroughput: 1_250_000, uploadThroughput: 375_000 },
  "Slow 4G de Lighthouse (1,6 Mbps, 150 ms)": { latency: 150, downloadThroughput: 200_000, uploadThroughput: 93_750 },
};
const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];
for (const [name, net] of Object.entries(profiles)) {
  const cold = [], bytes = [];
  for (let i = 0; i < 5; i++) {
    const context = await browser.newContext({ ...devices["Pixel 7"] });
    const page = await context.newPage();
    const client = await context.newCDPSession(page);
    await client.send("Network.enable");
    await client.send("Network.setCacheDisabled", { cacheDisabled: true });
    let total = 0;
    client.on("Network.loadingFinished", (e) => (total += e.encodedDataLength));
    if (net) await client.send("Network.emulateNetworkConditions", { offline: false, ...net });
    const t0 = Date.now();
    await page.goto("http://127.0.0.1:47831/", { waitUntil: "commit" });
    await page.locator(".pairing h1").waitFor({ timeout: 60_000 });
    cold.push(Date.now() - t0);
    bytes.push(total);
    await context.close();
  }
  // Second visit, with the service worker installed: it must not need the network for the app.
  const context = await browser.newContext({ ...devices["Pixel 7"] });
  const page = await context.newPage();
  await page.goto("http://127.0.0.1:47831/");
  await page.locator(".pairing h1").waitFor();
  for (let i = 0; i < 30 && !(await page.evaluate(() => !!navigator.serviceWorker.controller)); i++) await page.waitForTimeout(300);
  await page.waitForTimeout(1500);
  const client = await context.newCDPSession(page);
  await client.send("Network.enable");
  if (net) await client.send("Network.emulateNetworkConditions", { offline: false, ...net });
  const t1 = Date.now();
  await page.reload({ waitUntil: "commit" });
  await page.locator(".pairing h1").waitFor({ timeout: 60_000 });
  const warm = Date.now() - t1;
  await context.close();
  console.log(`${name}\n   primera visita: ${median(cold)} ms (mediana de ${cold.join(", ")}) · ${Math.round(median(bytes) / 1024)} KiB recibidos\n   segunda visita (service worker): ${warm} ms`);
}
await browser.close();
run("stop");
