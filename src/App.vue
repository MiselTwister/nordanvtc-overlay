<script setup>
import { ref, onMounted, computed, watch } from "vue";
import { listen, emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PhysicalPosition } from "@tauri-apps/api/dpi"; // 🛠️ Required to manipulate OS window coordinates

const appWindow = getCurrentWindow(); 
const windowLabel = appWindow.label; 

const isBooting = ref(true);

const settings = ref({
  locked: true, 
  themeColor: '#0088ff', 
  opacity: 0.95,
  scale: 1.0,
  redline: 2500, 
  
  speedUnit: 'KMH', 
  tempUnit: 'C', 
  volUnit: 'L', 

  showDamage: 'SIMPLE', 
  showEcoDash: false,
  showWarnLights: true,
  showWaterTemp: true,
  showRpmText: true,
  showLimit: true,
  showGear: true,
  showCruise: true,
  showGpsOdo: true,
  showFuelBar: true
});

const truck = ref({ 
  sdkActive: false,
  speed: 0, limit: 0, gear: 0, fuel: 0, fuelRange: 0, fuelAvgCons: 0, temp: 0, 
  damageEngine: 0, damageTrans: 0, damageCabin: 0, damageChassis: 0, damageWheels: 0,
  rpm: 0, cruiseControl: 0, odometer: 0, routeDistance: 0, routeTime: 0,
  parkBrake: false, airPressureEmerg: false, oilPressWarning: false, waterTempWarning: false, battVoltWarning: false
});

// 💾 LOAD SAVED SETTINGS FROM DISK
const savedSettings = localStorage.getItem('nordan_settings');
if (savedSettings) {
  try {
    settings.value = { ...settings.value, ...JSON.parse(savedSettings) };
  } catch(e) {}
}

if (windowLabel === 'command') {
  // 💾 SAVE SETTINGS TO DISK WHENEVER CHANGED
  watch(settings, (newSettings) => {
    localStorage.setItem('nordan_settings', JSON.stringify(newSettings));
    emitTo('main', 'cmd-sync-settings', newSettings);
  }, { deep: true });
}

// Drag & Resize
const startDrag = () => { if (!settings.value.locked) appWindow.startDragging(); };
let isResizing = false; let startX = 0; let initialScale = 1.0;
const startResize = (e) => {
  if (settings.value.locked) return;
  isResizing = true; startX = e.clientX; initialScale = settings.value.scale;
  window.addEventListener('mousemove', handleResize);
  window.addEventListener('mouseup', stopResize);
};
const handleResize = (e) => {
  if (!isResizing) return;
  let newScale = initialScale + ((e.clientX - startX) * 0.002);
  settings.value.scale = Math.max(0.5, Math.min(newScale, 1.5));
};
const stopResize = () => { isResizing = false; window.removeEventListener('mousemove', handleResize); window.removeEventListener('mouseup', stopResize); };

// 💾 SAVE WINDOW POSITION WHEN USER LOCKS THE HUD
watch(() => settings.value.locked, async (isLocked) => {
  if (windowLabel === 'main' && isLocked) {
    try {
      const pos = await appWindow.outerPosition();
      localStorage.setItem('nordan_hud_x', pos.x.toString());
      localStorage.setItem('nordan_hud_y', pos.y.toString());
    } catch(e) {}
  }
});

// Auto-Hide based on Game Activity
watch([() => truck.value.sdkActive, () => settings.value.locked], async ([isActive, isLocked]) => {
  if (windowLabel === 'main') {
    if (isActive || !isLocked) {
      await appWindow.show();
    } else {
      await appWindow.hide();
    }
  }
}, { immediate: true });

// --- MATH CONVERSIONS ---
const useMph = computed(() => settings.value.speedUnit === 'MPH');
const displaySpeed = computed(() => Math.abs(truck.value.speed * (useMph.value ? 2.23694 : 3.6)));
const distMult = computed(() => useMph.value ? 0.621371 : 1);
const displayLimit = computed(() => Math.abs(truck.value.limit * 3.6 * distMult.value));
const displayDist = computed(() => truck.value.routeDistance * distMult.value);
const displayCruise = computed(() => truck.value.cruiseControl * 3.6 * distMult.value);
const displayRange = computed(() => truck.value.fuelRange * distMult.value);
const displayOdo = computed(() => truck.value.odometer * distMult.value);
const isSpeeding = computed(() => displayLimit.value > 0 && displaySpeed.value > displayLimit.value + 2);
const displayTemp = computed(() => settings.value.tempUnit === 'F' ? (truck.value.temp * 9/5) + 32 : truck.value.temp);

const displayCons = computed(() => {
  if (useMph.value) {
    return truck.value.fuelAvgCons > 0 ? (235.215 / (truck.value.fuelAvgCons * 100)).toFixed(1) : '0.0';
  }
  return (truck.value.fuelAvgCons * 100).toFixed(1); 
});

const isLowFuel = computed(() => truck.value.fuel < 15);
const simpleDamage = computed(() => Math.max(truck.value.damageEngine, truck.value.damageTrans, truck.value.damageCabin, truck.value.damageChassis, truck.value.damageWheels));
const hasDamage = computed(() => simpleDamage.value > 5);

const rpmPercent = computed(() => Math.min(100, Math.max(0, (truck.value.rpm / settings.value.redline) * 100)));
const shiftFlash = computed(() => rpmPercent.value > 95); 

const gearDisplay = computed(() => {
  if (truck.value.gear === 0) return 'N';
  if (truck.value.gear < 0) return 'R';
  return truck.value.gear.toString();
});

const speedParts = computed(() => {
  const spdStr = String(Math.abs(Math.round(displaySpeed.value))).padStart(3, '0');
  if (spdStr === '000') return { dimmed: '00', active: '0' };
  const activeMatch = spdStr.match(/^[0]*(.+)$/);
  const activePart = activeMatch ? activeMatch[1] : '0';
  const dimmedPart = spdStr.substring(0, spdStr.length - activePart.length);
  return { dimmed: dimmedPart, active: activePart };
});

const etaDisplay = computed(() => {
  if (truck.value.routeTime <= 0) return '--:--';
  const h = Math.floor(truck.value.routeTime / 3600);
  const m = Math.floor((truck.value.routeTime % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
});

const hudStyles = computed(() => ({
  '--brand-color': settings.value.themeColor,
  '--brand-glow': `${settings.value.themeColor}b3`, 
  '--bg-opacity': settings.value.opacity,
  '--hud-scale': settings.value.scale
}));

onMounted(async () => {
  setTimeout(() => { isBooting.value = false; }, 800);
  
  if (windowLabel === 'main') {
    // 💾 LOAD HUD POSITION ON BOOT
    try {
      const x = localStorage.getItem('nordan_hud_x');
      const y = localStorage.getItem('nordan_hud_y');
      if (x !== null && y !== null) {
        await appWindow.setPosition(new PhysicalPosition(parseInt(x), parseInt(y)));
      }
    } catch(e) { console.warn("Failed to load position", e); }

    try {
      await listen("telemetry-update", (event) => { truck.value = event.payload; });
      await listen("cmd-sync-settings", (event) => { settings.value = event.payload; });
    } catch (err) { console.error("Bridge Error:", err); }
  }
});
</script>

<template>
  <div v-if="windowLabel === 'main'" class="hud-wrapper" :style="hudStyles" :class="{ 'hud-hidden': !truck.sdkActive && settings.locked }">
    
    <div class="hud-container" :class="{ 'boot-sequence': isBooting }">
      
      <div class="integrated-shift-bar">
        <div v-for="i in 10" :key="i" class="shift-led" 
             :class="{
               'led-green': rpmPercent > (i * 9) && i <= 4 && !shiftFlash,
               'led-yellow': rpmPercent > (i * 9) && i > 4 && i <= 8 && !shiftFlash,
               'led-red': rpmPercent > (i * 9) && i > 8 && !shiftFlash,
               'led-flash': shiftFlash
             }">
        </div>
      </div>
      
      <div class="drag-handle" @mousedown="startDrag" :class="{ 'locked-handle': settings.locked }">
        <div class="drag-grip" v-if="!settings.locked"></div>
        <div class="lock-icon" v-else>🔒</div>
      </div>

      <div class="hud-layout">
        <div class="hud-zone recessed-screen panel-left" v-if="settings.showDamage !== 'OFF' || settings.showWaterTemp || settings.showRpmText">
          <div class="unskew inner-grid">
            <div class="adv-damage-grid" v-if="settings.showDamage === 'ADVANCED'">
               <div class="adv-dmg-item" :class="{'danger-text': truck.damageEngine > 5}">ENG: {{Math.round(truck.damageEngine)}}%</div>
               <div class="adv-dmg-item" :class="{'danger-text': truck.damageTrans > 5}">TRN: {{Math.round(truck.damageTrans)}}%</div>
               <div class="adv-dmg-item" :class="{'danger-text': truck.damageChassis > 5}">CHS: {{Math.round(truck.damageChassis)}}%</div>
               <div class="adv-dmg-item" :class="{'danger-text': truck.damageCabin > 5}">CAB: {{Math.round(truck.damageCabin)}}%</div>
            </div>
            <div class="stat-row" v-if="settings.showDamage === 'SIMPLE'">
              <span class="label">DMG</span>
              <span class="value md" :class="{ 'danger-text flash-fast': hasDamage }">{{ Math.round(simpleDamage) }}<span class="unit-mini">%</span></span>
            </div>
            <div class="stat-row" v-if="settings.showWaterTemp">
              <span class="label">H2O</span>
              <span class="value md">{{ Math.round(displayTemp) }}<span class="unit-mini">°{{settings.tempUnit}}</span></span>
            </div>
            <div class="stat-row" v-if="settings.showRpmText">
              <span class="label">RPM</span>
              <span class="value md">{{ Math.round(truck.rpm) }}</span>
            </div>
          </div>
        </div>

        <div class="hud-zone center-core">
          <div class="warning-cluster" v-if="settings.showWarnLights">
            <div class="warn-icon" :class="{'warn-active warn-yellow': truck.parkBrake}">Ⓟ</div>
            <div class="warn-icon" :class="{'warn-active warn-red': truck.airPressureEmerg}">💨</div>
            <div class="warn-icon" :class="{'warn-active warn-red': truck.oilPressWarning}">🛢️</div>
            <div class="warn-icon" :class="{'warn-active warn-red': truck.waterTempWarning}">🌡️</div>
            <div class="warn-icon center-span" :class="{'warn-active warn-red': truck.battVoltWarning}">🔋</div>
          </div>
          <div class="speed-module" :class="{ 'speed-shake': isSpeeding }">
            <div class="speed-lcd">
              <span class="lcd-dimmed">{{ speedParts.dimmed }}</span><span class="lcd-active" :class="{ 'danger-text': isSpeeding }">{{ speedParts.active }}</span>
            </div>
            <span class="speed-unit">{{ settings.speedUnit }}</span>
          </div>
          <div class="limit-shield" v-if="settings.showLimit && displayLimit > 0">
            <span class="limit-val">{{ Math.round(displayLimit) }}</span>
          </div>
          <div class="gear-module" v-if="settings.showGear">
            <span class="gear-value" :class="{ 'text-neutral': truck.gear === 0, 'text-reverse': truck.gear < 0 }">{{ gearDisplay }}</span>
          </div>
        </div>

        <div class="hud-zone recessed-screen panel-right" v-if="settings.showGpsOdo || settings.showCruise || settings.showFuelBar || settings.showEcoDash">
          <div class="unskew inner-grid-right">
            <div class="nav-stack">
              <template v-if="truck.routeDistance > 0 && settings.showGpsOdo">
                <div class="stat-row justify-end">
                  <span class="label">DST</span>
                  <span class="value md">{{ Math.round(displayDist) }}<span class="unit-mini">{{ useMph ? 'MI' : 'KM' }}</span></span>
                </div>
                <div class="stat-row justify-end">
                  <span class="label">ETA</span>
                  <span class="value md accent">{{ etaDisplay }}</span>
                </div>
              </template>
              <template v-else>
                <div class="stat-row justify-end" v-if="settings.showGpsOdo">
                  <span class="label">ODO</span>
                  <span class="value md">{{ Math.round(displayOdo).toLocaleString() }}</span>
                </div>
                <div class="stat-row justify-end" v-if="settings.showCruise">
                  <span class="label">CRS</span>
                  <span class="value md" :class="truck.cruiseControl > 0 ? 'accent' : 'lcd-dimmed'">{{ truck.cruiseControl > 0 ? Math.round(displayCruise) : 'OFF' }}</span>
                </div>
              </template>
            </div>

            <div class="fuel-module" v-if="settings.showEcoDash || settings.showFuelBar">
              <div v-if="settings.showEcoDash" class="eco-dash">
                <div class="stat-row justify-end"><span class="label">RNG</span><span class="value sm">{{ Math.round(displayRange) }} {{useMph?'MI':'KM'}}</span></div>
                <div class="stat-row justify-end"><span class="label">AVG</span><span class="value sm">{{ displayCons }} {{useMph?'MPG':'L/100'}}</span></div>
              </div>
              <template v-if="settings.showFuelBar">
                <span class="label">FUEL</span>
                <div class="fuel-segmented">
                   <div v-for="i in 10" :key="i" class="fuel-segment" :class="{ 'seg-active': truck.fuel >= i * 10, 'seg-danger': isLowFuel && truck.fuel >= i * 10 }"></div>
                </div>
              </template>
            </div>

          </div>
        </div>
      </div>
      <div class="resize-handle" v-if="!settings.locked" @mousedown="startResize" title="Drag to Resize HUD">⇲</div>
    </div>
  </div>

  <div v-else class="cmd-container">
    <div class="cmd-header">
      <div class="logo-area">
        <div class="logo-dot"></div>
        <h2>NORDAN COMMAND</h2>
      </div>
      <div class="status-badge" :class="truck.sdkActive ? 'online' : 'offline'">
        {{ truck.sdkActive ? '● LINKED' : '○ WAITING' }}
      </div>
    </div>

    <div class="cmd-body scrollable">
      
      <div class="cmd-section">
        <h3>APPEARANCE</h3>
        <div class="control-group">
          <span class="cmd-label">ACCENT COLOR</span>
          <div class="color-picker">
            <div class="color-swatch" style="background: #0088ff;" @click="settings.themeColor = '#0088ff'" :class="{'active-swatch': settings.themeColor === '#0088ff'}"></div>
            <div class="color-swatch" style="background: #ff2a00;" @click="settings.themeColor = '#ff2a00'" :class="{'active-swatch': settings.themeColor === '#ff2a00'}"></div>
            <div class="color-swatch" style="background: #00fa9a;" @click="settings.themeColor = '#00fa9a'" :class="{'active-swatch': settings.themeColor === '#00fa9a'}"></div>
            <div class="color-swatch" style="background: #ff9900;" @click="settings.themeColor = '#ff9900'" :class="{'active-swatch': settings.themeColor === '#ff9900'}"></div>
            <div class="color-swatch" style="background: #b829ff;" @click="settings.themeColor = '#b829ff'" :class="{'active-swatch': settings.themeColor === '#b829ff'}"></div>
          </div>
        </div>
        <div class="control-group">
          <span class="cmd-label">BACKGROUND OPACITY ({{ Math.round(settings.opacity * 100) }}%)</span>
          <input type="range" min="0" max="1" step="0.05" v-model.number="settings.opacity" class="pro-slider">
        </div>
      </div>

      <div class="cmd-section">
        <h3>HUD MODULES</h3>
        <div class="module-grid">
          <button class="toggle-btn" :class="{ 'btn-on': settings.showWarnLights }" @click="settings.showWarnLights = !settings.showWarnLights"><span class="indicator"></span> Warning Cluster</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showEcoDash }" @click="settings.showEcoDash = !settings.showEcoDash"><span class="indicator"></span> Eco Dash (Range)</button>
          
          <button class="toggle-btn" :class="{ 'btn-on': settings.showWaterTemp }" @click="settings.showWaterTemp = !settings.showWaterTemp"><span class="indicator"></span> H2O Temp</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showRpmText }" @click="settings.showRpmText = !settings.showRpmText"><span class="indicator"></span> RPM Num</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showLimit }" @click="settings.showLimit = !settings.showLimit"><span class="indicator"></span> Speed Limit</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showGear }" @click="settings.showGear = !settings.showGear"><span class="indicator"></span> Gear Box</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showCruise }" @click="settings.showCruise = !settings.showCruise"><span class="indicator"></span> Cruise Ctrl</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showGpsOdo }" @click="settings.showGpsOdo = !settings.showGpsOdo"><span class="indicator"></span> GPS/Odo</button>
          <button class="toggle-btn" :class="{ 'btn-on': settings.showFuelBar }" @click="settings.showFuelBar = !settings.showFuelBar"><span class="indicator"></span> Fuel Bar</button>
        </div>

        <div class="control-group mt-10">
          <span class="cmd-label">DAMAGE DISPLAY MODE</span>
          <div class="segmented-control">
            <button :class="{'seg-on': settings.showDamage === 'OFF'}" @click="settings.showDamage = 'OFF'">OFF</button>
            <button :class="{'seg-on': settings.showDamage === 'SIMPLE'}" @click="settings.showDamage = 'SIMPLE'">SIMPLE</button>
            <button :class="{'seg-on': settings.showDamage === 'ADVANCED'}" @click="settings.showDamage = 'ADVANCED'">ADVANCED</button>
          </div>
        </div>
      </div>

      <div class="cmd-section">
        <h3>VEHICLE CALIBRATION & UNITS</h3>
        <div class="control-group">
          <span class="cmd-label">SHIFT LIGHT REDLINE ({{ settings.redline }} RPM)</span>
          <input type="range" min="1500" max="3500" step="50" v-model.number="settings.redline" class="pro-slider">
        </div>
        
        <div class="toggle-grid mt-10">
          <div class="control-group">
            <span class="cmd-label">SPEED</span>
            <div class="segmented-control">
              <button :class="{'seg-on': settings.speedUnit === 'KMH'}" @click="settings.speedUnit = 'KMH'">KM/H</button>
              <button :class="{'seg-on': settings.speedUnit === 'MPH'}" @click="settings.speedUnit = 'MPH'">MPH</button>
            </div>
          </div>
          <div class="control-group">
            <span class="cmd-label">TEMP</span>
            <div class="segmented-control">
              <button :class="{'seg-on': settings.tempUnit === 'C'}" @click="settings.tempUnit = 'C'">°C</button>
              <button :class="{'seg-on': settings.tempUnit === 'F'}" @click="settings.tempUnit = 'F'">°F</button>
            </div>
          </div>
        </div>
      </div>

      <div class="cmd-section">
        <button class="cmd-btn" :class="{ 'btn-danger': settings.locked }" @click="settings.locked = !settings.locked">
          {{ settings.locked ? '🔒 HUD FULLY LOCKED' : '🔓 HUD EDITABLE (Drag & Resize Active)' }}
        </button>
      </div>

    </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@600;700;800&family=Rajdhani:wght@600;700&display=swap');

:root {
  --brand-color: #0088ff;
  --brand-glow: rgba(0, 136, 255, 0.8);
  --bg-opacity: 0.95;
  --hud-scale: 1.0;
  --danger-red: #ff2a00;
  --danger-glow: rgba(255, 42, 0, 1);
  --warn-yellow: #ffcc00;
  --safe-green: #00fa9a;
  --text-main: #ffffff;
  --text-dim: #5c677d;
  --font-digital: 'Chakra Petch', sans-serif;
  --font-label: 'Rajdhani', sans-serif;
}

body { margin: 0; overflow: hidden; background: transparent; user-select: none; font-family: 'Segoe UI', sans-serif; }

.hud-wrapper { width: 100vw; height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; transform: scale(var(--hud-scale)); transform-origin: center center; transition: opacity 0.3s; }
.hud-hidden { opacity: 0 !important; pointer-events: none !important; }

/* 🚨 RECESSED WARNING CLUSTER - MADE MORE VISIBLE */
.warning-cluster { 
  background: linear-gradient(135deg, rgba(10,12,16,0.8), rgba(4,5,8,0.9));
  height: 65px; 
  padding: 0 15px;
  display: grid; 
  grid-template-columns: 1fr 1fr;
  align-content: center;
  justify-items: center;
  gap: 6px 15px;
  border-radius: 6px; 
  border: 1px solid rgba(255,255,255,0.05); 
  box-shadow: inset 0 5px 15px rgba(0,0,0,0.8); 
  transform: skewX(-12deg); 
  transition: 0.3s;
}
.warn-icon { 
  font-size: 14px; 
  opacity: 0.3; /* 🛠️ FIXED: Increased opacity so the inactive icons are clearly visible */
  filter: grayscale(100%); 
  transition: 0.3s; 
  transform: skewX(12deg); 
  line-height: 1;
}
.center-span { grid-column: span 2; } 
.warn-active { 
  opacity: 1 !important; 
  filter: none !important; 
  animation: flash 0.5s infinite alternate; 
}
.warn-yellow { text-shadow: 0 0 15px var(--warn-yellow); color: var(--warn-yellow); }
.warn-red { text-shadow: 0 0 15px var(--danger-red); color: var(--danger-red); }

/* --- THE EDGE-TO-EDGE CONTAINER --- */
.hud-container { 
  width: 100vw; 
  height: 100vh; 
  background: linear-gradient(to bottom, rgba(10, 12, 16, var(--bg-opacity)), rgba(6, 8, 12, var(--bg-opacity))), repeating-linear-gradient(45deg, rgba(255,255,255,0.015) 0px, rgba(255,255,255,0.015) 2px, transparent 2px, transparent 4px); 
  backdrop-filter: blur(20px); 
  -webkit-backdrop-filter: blur(20px); 
  border-bottom: 2px solid rgba(255, 255, 255, 0.05); 
  display: flex; 
  position: relative; 
  pointer-events: none !important; 
  transition: transform 0.6s cubic-bezier(0.16, 1, 0.3, 1); 
}
.boot-sequence { opacity: 0; transform: translateY(40px) scale(0.95); }

.integrated-shift-bar { position: absolute; top: 0; left: 0; right: 0; height: 6px; display: flex; gap: 2px; background: #000; padding: 0 10%; border-radius: 0; overflow: hidden;}
.shift-led { flex: 1; height: 100%; background: #111; transition: 0.05s; }
.led-green { background: var(--safe-green); box-shadow: 0 0 15px var(--safe-green); }
.led-yellow { background: var(--warn-yellow); box-shadow: 0 0 15px var(--warn-yellow); }
.led-red { background: var(--danger-red); box-shadow: 0 0 15px var(--danger-red); }
.led-flash { background: var(--brand-color); box-shadow: 0 0 20px var(--brand-glow); animation: flash 0.08s infinite alternate; }

.drag-handle { width: 30px; height: 100%; display: flex; align-items: center; justify-content: center; pointer-events: auto !important; cursor: grab; border-right: 1px solid rgba(255, 255, 255, 0.05); transition: 0.3s; border-radius: 0;}
.drag-handle:active { cursor: grabbing; }
.drag-grip { width: 6px; height: 40px; background: repeating-linear-gradient(0deg, var(--text-dim) 0px, var(--text-dim) 2px, transparent 2px, transparent 6px); opacity: 0.5; }
.locked-handle { background: rgba(255, 0, 0, 0.1); cursor: not-allowed; border-right: 1px solid rgba(255, 0, 0, 0.3); }
.lock-icon { font-size: 12px; opacity: 0.5; }
.resize-handle { position: absolute; bottom: 5px; right: 5px; width: 30px; height: 30px; color: var(--brand-color); font-size: 20px; font-weight: bold; pointer-events: auto !important; cursor: nwse-resize; display: flex; justify-content: center; align-items: center; text-shadow: 0 0 10px var(--brand-glow); transition: 0.2s;}

.hud-layout { display: flex; flex: 1; justify-content: center; align-items: center; padding: 0 30px; gap: 30px;}
.recessed-screen { background: rgba(4, 5, 8, var(--bg-opacity)); padding: 8px 25px; border: 1px solid rgba(255, 255, 255, 0.03); box-shadow: inset 0 5px 15px rgba(0,0,0,0.8), 0 1px 0 rgba(255,255,255,0.05); min-width: 150px; }
.panel-left { transform: skewX(-15deg); border-left: 3px solid var(--brand-color); border-radius: 6px 0 0 6px; }
.panel-right { transform: skewX(15deg); border-right: 3px solid var(--brand-color); border-radius: 0 6px 6px 0; }
.unskew { transform: skewX(15deg); }
.panel-right .unskew { transform: skewX(-15deg); }

.stat-row { display: flex; justify-content: space-between; align-items: flex-end; width: 100%; gap: 15px; }
.justify-end { justify-content: flex-end; }
.label { font-family: var(--font-label); font-size: 13px; font-weight: 700; color: var(--text-dim); letter-spacing: 2px; }
.value.md { font-family: var(--font-digital); font-size: 22px; font-weight: 700; color: var(--text-main); line-height: 0.9; text-shadow: 0 0 10px rgba(255,255,255,0.2); }
.value.sm { font-family: var(--font-digital); font-size: 16px; font-weight: 700; color: var(--text-main); }
.unit-mini { font-size: 12px; color: var(--text-dim); margin-left: 2px; }
.accent { color: var(--brand-color) !important; text-shadow: 0 0 15px var(--brand-glow) !important; transition: 0.3s; }

/* Advanced Damage */
.adv-damage-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 15px; margin-bottom: 4px;}
.adv-dmg-item { font-family: var(--font-digital); font-size: 10px; font-weight: bold; color: var(--text-dim); }

.center-core { display: flex; align-items: center; gap: 25px; }
.speed-module { display: flex; flex-direction: column; align-items: center; margin-top: -5px; }
.speed-lcd { font-family: var(--font-digital); font-size: 82px; font-weight: 800; line-height: 0.75; letter-spacing: -2px; }
.lcd-dimmed { color: rgba(255, 255, 255, 0.08); }
.lcd-active { color: #fff; text-shadow: 0 0 25px rgba(255,255,255,0.3); }
.speed-unit { font-family: var(--font-label); font-size: 16px; font-weight: 700; color: var(--brand-color); letter-spacing: 6px; margin-top: 8px; text-shadow: 0 0 10px var(--brand-glow); transition: 0.3s; }

.limit-shield { display: flex; justify-content: center; align-items: center; width: 44px; height: 44px; background: #fff; border: 5px solid var(--danger-red); border-radius: 50%; box-shadow: 0 5px 15px rgba(0,0,0,0.6); margin: 0 10px; }
.limit-val { font-family: var(--font-digital); color: #000; font-size: 20px; font-weight: 800; line-height: 1; }

.gear-module { background: linear-gradient(135deg, rgba(255,255,255,0.05), rgba(255,255,255,0.01)); width: 65px; height: 75px; display: flex; justify-content: center; align-items: center; border-radius: 8px; border: 2px solid var(--brand-color); box-shadow: inset 0 0 20px var(--brand-glow), 0 0 20px var(--brand-glow); transform: skewX(-12deg); transition: 0.3s; }
.gear-value { font-family: var(--font-digital); font-size: 56px; font-weight: 800; color: #fff; transform: skewX(12deg); text-shadow: 0 0 15px #fff; }
.text-neutral { color: var(--safe-green); text-shadow: 0 0 15px var(--safe-green); }
.text-reverse { color: var(--danger-red); text-shadow: 0 0 15px var(--danger-red); }

.fuel-module { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; }
.eco-dash { display: flex; flex-direction: column; gap: 2px; border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 5px; margin-bottom: 2px;}
.fuel-segmented { display: flex; gap: 4px; }
.fuel-segment { width: 8px; height: 22px; background: rgba(255,255,255,0.06); transform: skewX(-15deg); transition: 0.3s; border-radius: 1px; }
.seg-active { background: #fff; box-shadow: 0 0 8px rgba(255,255,255,0.6); }
.seg-danger { background: var(--danger-red); box-shadow: 0 0 12px var(--danger-glow); animation: flash 0.5s infinite; }

.danger-text { color: var(--danger-red) !important; text-shadow: 0 0 20px var(--danger-glow) !important; }
.flash-fast { animation: flash 0.2s infinite alternate; }
.speed-shake { animation: shake 0.2s infinite; }
@keyframes flash { 0% { opacity: 0.3; } 100% { opacity: 1; } }
@keyframes shake { 0%, 100% { transform: translateX(0); } 25% { transform: translateX(-1px); } 75% { transform: translateX(1px); } }

.cmd-container { width: 100vw; height: 100vh; background: #0b0c10; color: white; display: flex; flex-direction: column; overflow: hidden; }
.cmd-header { background: #13151c; padding: 20px; border-bottom: 2px solid #333; display: flex; justify-content: space-between; align-items: center; box-shadow: 0 5px 15px rgba(0,0,0,0.5); z-index: 10; }
.logo-area { display: flex; align-items: center; gap: 10px; }
.logo-dot { width: 12px; height: 12px; background: var(--brand-color); border-radius: 50%; box-shadow: 0 0 10px var(--brand-color); transition: 0.3s;}
.cmd-header h2 { margin: 0; font-family: var(--font-digital); font-size: 20px; letter-spacing: 2px; }
.status-badge { font-size: 11px; font-weight: bold; padding: 4px 8px; border-radius: 4px; border: 1px solid currentColor; }
.online { background: rgba(0, 250, 154, 0.1); color: var(--safe-green); }
.offline { background: rgba(255, 255, 255, 0.1); color: #888; }
.scrollable { overflow-y: auto; padding: 20px; display: flex; flex-direction: column; gap: 25px; }
.cmd-section { background: #171921; border-radius: 8px; padding: 20px; border: 1px solid #222; }
.cmd-section h3 { margin: 0 0 15px 0; font-size: 12px; color: #666; letter-spacing: 2px; }
.control-group { display: flex; flex-direction: column; gap: 10px; margin-bottom: 15px; }
.cmd-label { font-size: 11px; font-weight: bold; color: #aaa; }
.mt-10 { margin-top: 10px; }
.color-picker { display: flex; gap: 10px; }
.color-swatch { width: 35px; height: 35px; border-radius: 50%; cursor: pointer; border: 2px solid transparent; transition: 0.2s; box-shadow: 0 4px 10px rgba(0,0,0,0.5); }
.active-swatch { border-color: white; transform: scale(1.1); }
.pro-slider { -webkit-appearance: none; width: 100%; height: 6px; background: #222; border-radius: 3px; outline: none; }
.pro-slider::-webkit-slider-thumb { -webkit-appearance: none; appearance: none; width: 16px; height: 16px; border-radius: 50%; background: #fff; cursor: pointer; }
.module-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.toggle-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 15px; }
.toggle-btn { background: #1f222d; border: 1px solid #333; color: white; padding: 12px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 10px; font-weight: bold; font-size: 11px; transition: 0.2s; text-transform: uppercase; }
.indicator { width: 8px; height: 8px; border-radius: 50%; background: #444; flex-shrink: 0;}
.btn-on { border-color: var(--brand-color); background: rgba(255, 255, 255, 0.05); }
.btn-on .indicator { background: var(--brand-color); box-shadow: 0 0 8px var(--brand-color); }
.segmented-control { display: flex; background: #1f222d; border-radius: 6px; padding: 4px; }
.segmented-control button { flex: 1; background: transparent; border: none; color: #888; padding: 8px; border-radius: 4px; cursor: pointer; font-weight: bold; transition: 0.2s; }
.seg-on { background: #333 !important; color: white !important; box-shadow: 0 2px 5px rgba(0,0,0,0.3); }
.cmd-btn { width: 100%; background: #1f222d; color: white; border: 1px solid #333; padding: 15px; border-radius: 6px; font-size: 13px; font-weight: bold; cursor: pointer; transition: 0.2s; }
.btn-danger { background: rgba(255, 42, 0, 0.1); border-color: var(--danger-red); color: var(--danger-red); }
</style>