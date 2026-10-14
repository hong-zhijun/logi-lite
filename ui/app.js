'use strict';
const $ = id => document.getElementById(id);
const $$ = selector => [...document.querySelectorAll(selector)];
const invoke = window.__TAURI__?.core.invoke;
const BUTTONS = {back:'后退侧键',forward:'前进侧键',middle:'中键（鼠标滚轮）'};
const ACTIONS = new Set(['默认','禁用','打开 App','调度中心']);
let runtimeState = null, savedConfig = null, draftConfig = null, selectedButton = null, chosenAction = '默认';
let hardwareResult = {}, actual = null, draftHardware = null, verified = false, hardwareBusy = false;
let hardwareDraftEdited = false;
let currentPage = 'buttons', selectedPanel = null;
let topology = null, deviceGeneration = 0, saveBusy = false, recording = false, toastTimer, polling = false, lastProbe = 0;
const clone = value => JSON.parse(JSON.stringify(value));
function toast(message, error=false) {
  clearTimeout(toastTimer); $('toast').textContent = message; $('toast').classList.toggle('error',error); $('toast').hidden = false;
  toastTimer = setTimeout(() => {$('toast').hidden = true;},error?6500:3000);
}
async function native(request) {
  if (!invoke) throw new Error('请在 Logi Lite 应用中使用此功能。');
  const result = await invoke('runtime',{request});
  if (!result.ok) throw new Error(result.error || '操作失败');
  return result;
}
const ready = () => Boolean(runtimeState?.permissions?.accessibility && runtimeState?.permissions?.inputMonitoring && runtimeState?.permissions?.postEvents);
const mappingDirty = () => draftConfig && savedConfig && ['back','forward','middle'].some(key=>draftConfig[key]!==savedConfig[key]);
function commitState(id,state,label,icon,description) {
  const button=$(id);
  button.dataset.state=state;
  button.querySelector('.commit-label').textContent=label;
  button.querySelector('use').setAttribute('href',`#i-${icon}`);
  button.title=description;
  button.setAttribute('aria-busy',String(state==='busy'));
}
function updateDraftLabel() {
  const dirty=Boolean(mappingDirty());
  const state=saveBusy?'busy':!ready()?'unavailable':dirty?'pending':'saved';
  const label=saveBusy?'保存中…':!ready()?'请先授权':dirty?'保存并应用':'已保存';
  commitState('save-mapping',state,label,saveBusy?'refresh':state==='saved'?'check':'alert',saveBusy?'正在保存配置':!ready()?'请先完成必要授权':dirty?'有尚未应用的修改':'当前配置已保存');
  $('save-mapping').disabled=saveBusy||!ready()||!dirty;
}
function renderRuntime(result, initial=false) {
  const wasPermitted=ready();
  runtimeState = result;
  if(wasPermitted!==ready()){verified=false;topology=null;}

  // Runtime polling can update enabled state; never replace unsaved button edits.
  if (initial || !draftConfig) {savedConfig=clone(result.config);draftConfig=clone(result.config);renderAction();}
  else {savedConfig=clone(result.config);}
  $('mapping-toggle').checked = result.active;
  $('mapping-dot').className = 'status-dot' + (result.active?'':' pending');
  $('mapping-status').textContent = result.active?'映射已启用':'映射已暂停';
  $('mapping-status').title = result.inputStatus;
  $('switch-label').textContent = result.active?'映射已启用':'启用映射';
  $('permission-link').hidden = ready();
  for(const [statusId,buttonId,granted] of [['accessibility-status','open-accessibility',result.permissions.accessibility],['input-status','open-input',result.permissions.inputMonitoring]]) {
    $(statusId).textContent=granted?'已授权':'未授权';
    $(statusId).classList.toggle('permission-pending',!granted);
    $(statusId).classList.toggle('permission-granted',granted);
    $(buttonId).hidden=granted;
  }
  renderPermissions();
  updateDraftLabel();
}
function page(name) {
  currentPage=name;
  $$('.page').forEach(el=>el.hidden=el.id!==`page-${name}`);
  $('settings-toggle').setAttribute('aria-expanded',String(name==='settings'));
  $('settings-toggle').setAttribute('aria-label',name==='settings'?'返回鼠标':'打开设置');
  renderPermissions();
  stopRecording();
}
function renderPermissions() {
  const permitted=ready();
  const gated=!permitted&&currentPage!=='settings';
  $('permission-gate').hidden=!gated;
  $('device-content').classList.toggle('locked',gated);
  $('page-buttons').inert=!permitted;
  $('mapping-toggle').disabled=!permitted;
  $('gate-authorize').disabled=!runtimeState;
  renderRefreshControls();
  for(const [id,label,granted] of [['gate-accessibility','辅助功能',runtimeState?.permissions.accessibility],['gate-input','输入监控',runtimeState?.permissions.inputMonitoring]]) {
    $(id).textContent=`${label} · ${runtimeState?(granted?'已授权':'待授权'):'检查中'}`;
    $(id).classList.toggle('ready',Boolean(granted));
  }
  if(!permitted){$('connection-text').textContent=runtimeState?'等待授权':'正在检查权限…';$('connection-dot').className='status-dot pending';}
  if(gated)stopRecording();
}
function chooseButton(key) {
  if(key&&!ready())return;
  selectedButton=key;
  setPanel(key?'button':null);
  stopRecording();
  $$('[data-button]').forEach(el=>{el.classList.toggle('selected',el.dataset.button===key);el.setAttribute('aria-pressed',String(el.dataset.button===key));});
  if (!key) return;
  $('selected-title').textContent=BUTTONS[key];
  renderAction();
}
function setPanel(panel) {
  selectedPanel=panel;
  for(const [id,name] of [['button-editor','button'],['hardware-editor','hardware']]) {
    const open=panel===name;
    $(id).inert=!open;
    $(id).setAttribute('aria-hidden',String(!open));
    $(id).classList.toggle('open',open);
  }
  $('page-buttons').classList.toggle('no-selection',!panel);
  $('wheel-hotspot').classList.toggle('selected',panel==='hardware');
  $('wheel-hotspot').setAttribute('aria-pressed',String(panel==='hardware'));
  stopRecording();
}
function openHardware() {
  if(!ready())return;
  chooseButton(null);
  setPanel('hardware');
}
function renderAction() {
  if (!draftConfig || !selectedButton || !ready()) return;
  const value = draftConfig[selectedButton];
  chosenAction = ACTIONS.has(value)?value:'shortcut';
  $$('.action-option').forEach(el=>{el.classList.toggle('selected',el.dataset.action===chosenAction);el.setAttribute('aria-pressed',String(el.dataset.action===chosenAction));});
  $('shortcut-editor').hidden=chosenAction!=='shortcut';
  $('shortcut').value=chosenAction==='shortcut'?value:'';
  stopRecording();updateDraftLabel();
}
function chooseAction(value) {
  if (!draftConfig || !selectedButton || !ready()) return;
  const previous=draftConfig[selectedButton];
  draftConfig[selectedButton]=value==='shortcut'?(ACTIONS.has(previous)?'Cmd+C':previous):value;
  renderAction();
}
function stopRecording() {recording=false;$('record-shortcut').textContent='录制';}
const keyNames = {ArrowLeft:'Left',ArrowRight:'Right',ArrowUp:'Up',ArrowDown:'Down',Escape:'Esc',Enter:'Enter',' ':'Space',Backspace:'Backspace',Delete:'Delete',PageUp:'PageUp',PageDown:'PageDown',Home:'Home',End:'End',Tab:'Tab'};
function record(event) {
  if (!recording) return;
  event.preventDefault();event.stopPropagation();
  if (['Meta','Control','Alt','Shift'].includes(event.key)) return;
  if (event.key==='Escape'&&!event.metaKey&&!event.ctrlKey&&!event.altKey&&!event.shiftKey){stopRecording();return;}
  const key=keyNames[event.key] || (/^Key[A-Z]$/.test(event.code)?event.code.slice(3):/^Digit[0-9]$/.test(event.code)?event.code.slice(5):event.key.toUpperCase());
  const modifiers=[event.metaKey?'Cmd':null,event.ctrlKey?'Ctrl':null,event.altKey?'Option':null,event.shiftKey?'Shift':null].filter(Boolean);
  const supported = /^[A-Z0-9]$/.test(key)||/^F([1-9]|1[0-9]|20)$/.test(key)||Object.values(keyNames).includes(key);
  if (!supported || key==='Backspace') {toast('这个按键暂不支持，请使用字母、数字、方向键或 F1–F20。',true);return;}
  draftConfig[selectedButton]=[...modifiers,key].join('+');renderAction();
}
async function saveMapping() {
  if (saveBusy||!draftConfig||!ready()||!mappingDirty()) return;
  saveBusy=true;updateDraftLabel();
  const submitted=clone(draftConfig);submitted.enabled=runtimeState.active;
  try {
    const result=await native({action:'save',config:submitted});
    renderRuntime(result);toast('按键配置已保存'+(result.active?'并应用':''));
  } catch(error){toast(error.message,true);}
  finally {saveBusy=false;updateDraftLabel();}
}
async function toggleMapping() {
  $('mapping-toggle').disabled=true;
  try {const result=await native({action:'toggle'});renderRuntime(result);if (result.active && mappingDirty()) toast('映射已启用；按键修改仍需保存。');}
  catch(error){toast(error.message,true);$('mapping-toggle').checked=runtimeState.active;if (!ready())page('settings');}
  finally {$('mapping-toggle').disabled=false;}
}
function hardwareDirty() {
  if (!actual||!draftHardware) return false;
  return (actual.capabilities.dpi&&draftHardware.dpi!==actual.dpi)||(actual.capabilities.scroll&&draftHardware.scroll!==actual.scroll?.mode)||(draftHardware.scroll==='smart_shift'&&draftHardware.sensitivity!==actual.scroll?.sensitivity);
}
function renderRefreshControls() {
  for(const id of ['refresh-hardware','refresh-device'])$(id).disabled=hardwareBusy||!ready();
  $('refresh-hardware').classList.toggle('refreshing',hardwareBusy);
  $('refresh-hardware').setAttribute('aria-busy',String(hardwareBusy));
}
function renderHardware() {
  const caps=actual?.capabilities || {};
  $('dpi-controls').disabled=hardwareBusy||!verified||!caps.dpi;
  $('scroll-controls').disabled=hardwareBusy||!verified||!caps.scroll;
  $$('[data-mode="smart_shift"]').forEach(el=>el.disabled=!caps.smart_shift);
  $('sensitivity-controls').hidden=draftHardware?.scroll!=='smart_shift';
  $('sensitivity-controls').disabled=hardwareBusy||!verified||!caps.smart_shift;
  if (draftHardware) {
    $('dpi-range').value=draftHardware.dpi??1000;$('dpi-value').textContent=draftHardware.dpi??'—';
    $('sensitivity').value=draftHardware.sensitivity??50;$('sensitivity-value').textContent=`${draftHardware.sensitivity??50}%`;
    $$('[data-mode]').forEach(el=>{el.classList.toggle('selected',el.dataset.mode===draftHardware.scroll);el.setAttribute('aria-pressed',String(el.dataset.mode===draftHardware.scroll));});
  }
  const dirty=Boolean(hardwareDirty());
  const state=hardwareBusy?'busy':!verified||!ready()?'unavailable':dirty?'pending':'saved';
  const label=hardwareBusy?'同步中…':!ready()?'请先授权':!verified?'设备暂不可用':dirty?'应用到鼠标':'已同步';
  commitState('apply-hardware',state,label,hardwareBusy?'refresh':state==='saved'?'check':'alert',hardwareBusy?'正在与鼠标通信':!ready()?'请先完成必要授权':!verified?'请刷新并确认设备状态':dirty?'有尚未应用的修改':'与鼠标当前设置一致');
  $('apply-hardware').disabled=hardwareBusy||!verified||!ready()||!dirty;
  renderRefreshControls();
  if (verified&&actual) {
    $('actual-dpi').textContent=`DPI ${actual.dpi??'—'}`;
    $('actual-scroll').textContent={ratchet:'段落滚动',free:'自由滚动',smart_shift:'SmartShift'}[actual.scroll?.mode]||'滚轮状态未知';
    $('diagnostic-summary').textContent=actual.warnings?.length?`已读取设备；${actual.warnings.length} 项能力诊断可在技术信息中查看。`:'设备响应正常，支持的设置已自动读取。';
  } else {
    $('actual-dpi').textContent='DPI —';
    $('actual-scroll').textContent=hardwareBusy?'正在读取…':'滚轮状态未知';
    const message=hardwareResult.error||'唤醒或重新连接鼠标后，可重试读取状态。';
    if(!hardwareBusy)$('diagnostic-summary').textContent=message;
  }
  $('diagnostic-json').textContent=JSON.stringify(hardwareResult,null,2);
}
function updateDeviceStatus(result) {
  const oldDirty=hardwareDirty()||hardwareDraftEdited;
  hardwareResult=result;
  verified=result.ok===true&&result.target_verified===true;
  if (result.status) {
    actual=result.status;
    const range=actual.dpi_ranges?.[0];
    const values=actual.dpi_choices||[];
    if (range) {
      for(const id of ['dpi-range']) {$(id).min=range.min;$(id).max=range.max;$(id).step=range.step;}
      $('dpi-min').textContent=range.min;$('dpi-max').textContent=range.max;
    } else if(values.length) {
      for(const id of ['dpi-range']) {$(id).min=Math.min(...values);$(id).max=Math.max(...values);$(id).step=values.length>1?Math.min(...values.slice(1).map((x,i)=>x-values[i]).filter(x=>x>0)):1;}
      $('dpi-min').textContent=Math.min(...values);$('dpi-max').textContent=Math.max(...values);
    }
    if (!oldDirty||!draftHardware) draftHardware={dpi:actual.dpi,scroll:actual.scroll?.mode||'ratchet',sensitivity:actual.scroll?.sensitivity??50};
    const battery=actual.battery;
    const levels={full:'满电',good:'电量较高',low:'电量偏低',critical:'电量极低',empty:'电量耗尽'};
    $('battery').textContent=battery?.percent!=null?`电量 ${battery.percent}%`:levels[battery?.level]||'电量未知';
  }
  $('connection-dot').className='status-dot'+(verified?'':' error');
  $('connection-text').textContent=verified?'蓝牙已连接':'设备暂不可用';
  renderHardware();
}
async function probe(showMessage=false) {
  if (hardwareBusy||runtimeState?.smoke||!invoke||!ready()) return;
  hardwareBusy=true;lastProbe=Date.now();renderHardware();
  const generation=deviceGeneration;
  try {
    const result=await invoke('hardware',{request:{action:'probe'}});
    if (generation!==deviceGeneration) return;
    updateDeviceStatus(result);
    if(showMessage)toast(result.ok?'设备状态已更新':result.error||'读取失败，请唤醒鼠标后重试。',!result.ok);
  } catch(error){verified=false;if(showMessage)toast(String(error),true);}
  finally {hardwareBusy=false;renderHardware();}
}
async function applyHardware() {
  if(hardwareBusy||!verified||!hardwareDirty()||!ready())return;
  const request={action:'apply'}, caps=actual.capabilities, submitted=clone(draftHardware);
  if(caps.dpi&&submitted.dpi!==actual.dpi)request.dpi=submitted.dpi;
  if(caps.scroll&&(submitted.scroll!==actual.scroll?.mode||(submitted.scroll==='smart_shift'&&submitted.sensitivity!==actual.scroll?.sensitivity))){
    request.scroll=submitted.scroll;
    if(submitted.scroll==='smart_shift')request.sensitivity=submitted.sensitivity;
  }
  hardwareBusy=true;renderHardware();
  const generation=deviceGeneration;
  try {
    const result=await invoke('hardware',{request});
    if(generation!==deviceGeneration){toast('连接已改变，请重新确认设备状态。',true);return;}
    updateDeviceStatus(result);
    if(result.ok){hardwareDraftEdited=false;draftHardware={dpi:actual.dpi,scroll:actual.scroll?.mode||'ratchet',sensitivity:actual.scroll?.sensitivity??50};toast('已应用，并确认鼠标实际设置');}
    else toast(result.error||'未能确认设置，请重试读取。',true);
  } catch(error){verified=false;toast(String(error),true);}
  finally{hardwareBusy=false;renderHardware();}
}
async function poll() {
  if(polling||!runtimeState||runtimeState.smoke)return;
  polling=true;
  try {
    const result=await native({action:'state'});renderRuntime(result);
    const metadata=await native({action:'metadata'});
    const targets=metadata.devices.filter(d=>d.hidpp_candidate);
    const next=JSON.stringify(targets.map(d=>d.registry_id).sort());
    if(next!==topology){
      topology=next;deviceGeneration++;verified=false;lastProbe=0;
      if(targets.length!==1){actual=null;if(!hardwareDraftEdited)draftHardware=null;$('connection-dot').className='status-dot pending';$('connection-text').textContent=targets.length?'连接了多只目标鼠标':'鼠标未连接';$('battery').textContent='电量 —';renderHardware();}
      else {if(!hardwareBusy)await probe();}
    } else if(targets.length===1&&!verified&&!hardwareBusy&&Date.now()-lastProbe>30000&&!document.hidden){await probe();}
  } catch(error){/* Temporary read failures remain recoverable without recurring dialogs. */}
  finally{polling=false;}
}
async function requestPermissions() {
  try {renderRuntime(await native({action:'requestPermissions'}));page('settings');toast('请在系统提示或系统设置中完成授权。');}
  catch(error){toast(error.message,true);}
}
async function smokeCheck() {
  await Promise.all($$('.mouse-picture img').map(img=>img.decode()));
  const original=clone(runtimeState);
  const fixture=clone(runtimeState);
  fixture.permissions={accessibility:false,inputMonitoring:false,postEvents:false};
  renderRuntime(fixture);
  chooseButton('back');
  const permissionGateBlocks=!$('permission-gate').hidden&&$('page-buttons').inert&&$('hardware-editor').inert&&selectedButton===null&&$('mapping-toggle').disabled;
  $('gate-settings').click();
  const settingsFitsWithoutScroll=$('page-settings').scrollHeight<=$('page-settings').clientHeight+1;
  const settingsAccessible=!$('page-settings').hidden&&!$('page-settings').inert&&$('permission-gate').hidden;
  const settingsScrollbarHidden=getComputedStyle($('page-settings')).scrollbarWidth==='none';
  page('buttons');
  fixture.permissions={accessibility:true,inputMonitoring:false,postEvents:true};renderRuntime(fixture);
  const partialPermissionActions=$('open-accessibility').hidden&&!$('open-input').hidden&&!$('accessibility-status').classList.contains('permission-pending')&&$('input-status').classList.contains('permission-pending');
  const partialPermissionBlocks=!$('permission-gate').hidden&&$('page-buttons').inert;
  fixture.permissions={accessibility:true,inputMonitoring:true,postEvents:true};renderRuntime(fixture);
  const permissionGreen=$('accessibility-status').classList.contains('permission-granted')&&$('input-status').classList.contains('permission-granted')&&!$('permission-badge');
  const grantedPermissionActionsHidden=$('open-accessibility').hidden&&$('open-input').hidden&&!$('accessibility-status').classList.contains('permission-pending')&&!$('input-status').classList.contains('permission-pending');
  const permissionUnlocks=$('permission-gate').hidden&&!$('page-buttons').inert&&!$('mapping-toggle').disabled;
  fixture.active=true;renderRuntime(fixture);
  const activeStatus=$('mapping-status').textContent==='映射已启用'&&$('mapping-toggle').checked;
  fixture.active=false;renderRuntime(fixture);
  const mappingStatusSynced=activeStatus&&$('mapping-status').textContent==='映射已暂停'&&!$('mapping-toggle').checked;
  $('mapping-settings').click();
  const mappingSettingsAccessible=!$('page-settings').hidden&&$('page-settings').contains($('mapping-toggle'));
  page('buttons');
  const compactLayout=!document.querySelector('.sidebar,.version,.footer,.button-tabs,.nav-label,#page-buttons .visual-heading')&&document.querySelector('.titlebar').textContent.trim()==='';
  const header=document.querySelector('.device-header').getBoundingClientRect();
  const workspace=document.querySelector('.workspace').getBoundingClientRect();
  const capsule=document.querySelector('.connection').getBoundingClientRect();
  const model=document.querySelector('.device-header h1').getBoundingClientRect();
  const style=getComputedStyle(document.documentElement);
  const edge=parseFloat(style.getPropertyValue('--edge')),titlebarHeight=parseFloat(style.getPropertyValue('--titlebar-height')),headerGap=parseFloat(style.getPropertyValue('--header-gap'));
  const balancedInsets=[header.left,innerWidth-header.right,innerHeight-workspace.bottom].every(value=>Math.abs(value-edge)<1)&&Math.abs(header.top-titlebarHeight-headerGap)<1;
  const statusCapsuleFits=capsule.left>model.right&&capsule.right<$('refresh-hardware').getBoundingClientRect().left&&$('refresh-hardware').getBoundingClientRect().right<$('settings-toggle').getBoundingClientRect().left&&capsule.top>=header.top&&capsule.bottom<=header.bottom&&document.querySelector('.connection').contains($('mapping-status'));
  const initialUnselected=selectedButton===null&&selectedPanel===null&&$('hardware-editor').inert&&$('button-editor').inert&&$$('[data-button].selected').length===0;
  const overviewImage=document.querySelector('.mouse-picture.side img').getBoundingClientRect();
  const mouseCentered=Math.abs(overviewImage.x+overviewImage.width/2-innerWidth/2)<1;
  document.querySelector('.hotspot[data-button="forward"]').click();
  const selectionWorks=$('selected-title').textContent==='前进侧键'&&!$('button-editor').inert;
  const motionAllowed=!matchMedia('(prefers-reduced-motion: reduce)').matches;
  const panelAnimated=!motionAllowed||getComputedStyle($('button-editor')).transitionDuration.split(',').some(value=>parseFloat(value)>0);
  // Let the opening transition finish before measuring the picture and its markers.
  await new Promise(resolve=>setTimeout(resolve,motionAllowed?360:0));
  const image=document.querySelector('.mouse-picture.side img');
  const hot=document.querySelector('.hotspot[data-button="back"]');
  const r=image.getBoundingClientRect(),h=hot.getBoundingClientRect();
  const hotspotAligned=Math.abs((h.x+h.width/2-r.x)/r.width-.28)<.015&&Math.abs((h.y+h.height/2-r.y)/r.height-.59)<.015;
  chooseButton('back');
  $('wheel-hotspot').click();
  const hardwareOpensFromMouse=selectedPanel==='hardware'&&!$('hardware-editor').inert&&$('button-editor').inert&&!$('page-buttons').hidden;
  const wheel=document.querySelector('#wheel-hotspot').getBoundingClientRect();
  const wheelHotspotAligned=Math.abs((wheel.x+wheel.width/2-r.x)/r.width-.60)<.015&&Math.abs((wheel.y+wheel.height/2-r.y)/r.height-.12)<.015;
  const callout=$('wheel-hotspot').querySelector('.hotspot-label').getBoundingClientRect();
  const middle=document.querySelector('[data-button="middle"] .hotspot-label').getBoundingClientRect();
  const visual=document.querySelector('.visual-area').getBoundingClientRect();
  const wheelCalloutFits=callout.left>=visual.left&&callout.right<=visual.right&&callout.top>=visual.top&&callout.bottom<=visual.bottom&&callout.top<middle.top;
  updateDeviceStatus({ok:true,target_verified:true,status:{capabilities:{dpi:true,scroll:true,smart_shift:true},dpi:1000,dpi_ranges:[{min:200,max:4000,step:50}],scroll:{mode:'smart_shift',sensitivity:50}}});
  const hardwareSavedButton=$('apply-hardware').dataset.state==='saved'&&$('apply-hardware').disabled;
  $('dpi-range').value='1500';$('dpi-range').dispatchEvent(new Event('input'));
  const sliderDraftSeparate=$('dpi-value').textContent==='1500'&&draftHardware.dpi===1500&&$('actual-dpi').textContent==='DPI 1000'&&$('actual-scroll').textContent==='SmartShift'&&!$('apply-hardware').disabled&&$('apply-hardware').dataset.state==='pending'&&!document.querySelector('#dpi-number,#hardware-editor #actual-hardware');
  updateDeviceStatus({ok:false,error:'设备已断开'});
  const staleHardwareHidden=$('actual-dpi').textContent==='DPI —'&&$('actual-scroll').textContent==='滚轮状态未知'&&$('dpi-controls').disabled;
  $('close-hardware').click();
  const hardwareDismissed=selectedPanel===null&&$('hardware-editor').inert;
  $('settings-toggle').click();
  const navigationWorks=!$('page-settings').hidden&&$('page-buttons').hidden&&$('settings-toggle').getAttribute('aria-expanded')==='true';
  const card=$('mapping-preferences').getBoundingClientRect(),baselineHeight=card.height;
  fixture.permissions={accessibility:false,inputMonitoring:false,postEvents:false};renderRuntime(fixture);
  const prompt=$('permission-link').getBoundingClientRect(),title=$('mapping-preferences').querySelector('h3').getBoundingClientRect();
  const permissionPromptInline=Math.abs(prompt.y+prompt.height/2-title.y-title.height/2)<1&&Math.abs($('mapping-preferences').getBoundingClientRect().height-baselineHeight)<1;
  fixture.permissions={accessibility:true,inputMonitoring:true,postEvents:true};renderRuntime(fixture);
  $('settings-toggle').click();
  chooseButton('back');
  const previous=draftConfig.back;
  draftConfig.back='Cmd+Shift+S';renderRuntime(clone(runtimeState));
  const draftPreserved=draftConfig.back==='Cmd+Shift+S'&&mappingDirty();
  const mappingPendingButton=$('save-mapping').dataset.state==='pending'&&!$('save-mapping').disabled;
  // Permission revocation must immediately block an already open editor.
  fixture.permissions={accessibility:false,inputMonitoring:true,postEvents:true};renderRuntime(fixture);
  const revocationBlocks=!$('permission-gate').hidden&&$('page-buttons').inert;
  fixture.permissions={accessibility:true,inputMonitoring:true,postEvents:true};renderRuntime(fixture);
  draftConfig.back=previous;renderAction();
  $('close-editor').click();
  chooseButton('middle');$('close-editor').click();
  const editorDismissed=selectedButton===null&&$('button-editor').inert;
  const closeButton=$('close-editor').getBoundingClientRect(),closeGlyph=$('close-editor').querySelector('svg').getBoundingClientRect();
  const closeGlyphCentered=Math.abs(closeButton.x+closeButton.width/2-closeGlyph.x-closeGlyph.width/2)<.5&&Math.abs(closeButton.y+closeButton.height/2-closeGlyph.y-closeGlyph.height/2)<.5;
  const scrollbarHidden=getComputedStyle($('button-editor')).scrollbarWidth==='none';
  renderRuntime(original);
  const checks={settingsFitsWithoutScroll,permissionGreen,hardwareSavedButton,mappingPendingButton,closeGlyphCentered,partialPermissionActions,grantedPermissionActionsHidden,wheelCalloutFits,sliderDraftSeparate,staleHardwareHidden,mouseCentered,hardwareOpensFromMouse,wheelHotspotAligned,hardwareDismissed,permissionPromptInline,appIconLoaded:runtimeState.appIconLoaded===true,compactLayout,balancedInsets,statusCapsuleFits,mappingStatusSynced,mappingSettingsAccessible,permissionGateBlocks,settingsAccessible,settingsScrollbarHidden,partialPermissionBlocks,permissionUnlocks,revocationBlocks,initialUnselected,selectionWorks,panelAnimated,hotspotAligned,navigationWorks,draftPreserved,editorDismissed,scrollbarHidden,mouseImageLoaded:image.naturalWidth>0};
  const report={ok:Object.values(checks).every(Boolean),...checks,windowVisible:true,frontendLoaded:true,mappingActive:runtimeState.active,configWritten:false,hardwareWrites:false,permissionRequested:false};
  await invoke('smoke_report',{report});
}
$$('[data-page]').forEach(el=>el.addEventListener('click',()=>page(el.dataset.page)));
$$('[data-button]').forEach(el=>el.addEventListener('click',()=>chooseButton(el.dataset.button)));
$$('[data-action]').forEach(el=>el.addEventListener('click',()=>chooseAction(el.dataset.action)));
$('shortcut').addEventListener('input',()=>{draftConfig[selectedButton]=$('shortcut').value;updateDraftLabel();});
$('record-shortcut').addEventListener('click',()=>{recording=!recording;$('record-shortcut').textContent=recording?'按下组合键…':'录制';if(recording)$('shortcut').focus();});
document.addEventListener('keydown',record,true);
$('close-editor').addEventListener('click',()=>chooseButton(null));
$('close-hardware').addEventListener('click',()=>chooseButton(null));
$('wheel-hotspot').addEventListener('click',openHardware);
$('settings-toggle').addEventListener('click',()=>page(currentPage==='settings'?'buttons':'settings'));
$('save-mapping').addEventListener('click',saveMapping);$('mapping-toggle').addEventListener('change',toggleMapping);
$('dpi-range').addEventListener('input',()=>{if(!draftHardware)return;draftHardware.dpi=Number($('dpi-range').value);hardwareDraftEdited=hardwareDirty();renderHardware();});
$$('[data-mode]').forEach(el=>el.addEventListener('click',()=>{if(!draftHardware)return;draftHardware.scroll=el.dataset.mode;hardwareDraftEdited=hardwareDirty();renderHardware();}));
$('sensitivity').addEventListener('input',()=>{if(!draftHardware)return;draftHardware.sensitivity=Number($('sensitivity').value);hardwareDraftEdited=hardwareDirty();renderHardware();});
$('apply-hardware').addEventListener('click',applyHardware);
for(const id of ['refresh-hardware','refresh-device'])$(id).addEventListener('click',()=>probe(true));
$('gate-authorize').addEventListener('click',requestPermissions);
$('gate-settings').addEventListener('click',()=>page('settings'));
$('mapping-settings').addEventListener('click',()=>page('settings'));
$('permission-link').addEventListener('click',()=>page('settings'));
$('open-accessibility').addEventListener('click',()=>native({action:'settings',pane:'accessibility'}).catch(e=>toast(e.message,true)));
$('open-input').addEventListener('click',()=>native({action:'settings',pane:'input'}).catch(e=>toast(e.message,true)));
$('export-diagnostics').addEventListener('click',()=>native({action:'export',hardware:hardwareResult}).catch(e=>toast(e.message,true)));
async function start() {
  try {
    renderPermissions();
    if(!invoke){throw new Error('这是界面预览。鼠标控制需要在 Logi Lite 应用中运行。');}
    const result=await native({action:'state'});renderRuntime(result,true);renderHardware();
    if(result.error)toast(result.error,true);
    if(result.smoke){await smokeCheck();return;}
    await poll();setInterval(poll,4000);
  } catch(error){toast(error.message,true);}
}
start();
