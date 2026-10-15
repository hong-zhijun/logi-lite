'use strict';
const $ = id => document.getElementById(id);
const $$ = selector => [...document.querySelectorAll(selector)];
const invoke = window.__TAURI__?.core.invoke;
const BUTTONS = {back:'后退侧键',forward:'前进侧键',middle:'中键（鼠标滚轮）'};
const ACTIONS = new Set(['默认','禁用','打开 App','调度中心','水平滚动']);
let actionCatalog = {categories:[]}, editorMode=null;
let selectedCategory='';
let lastRuntimeError='';
let runtimeState = null, savedConfig = null, draftConfig = null, selectedButton = null, chosenAction = '默认';
let hardwareResult = {}, actual = null, draftHardware = null, verified = false, hardwareBusy = false, batteryBusy=false, lastBatteryProbe=0;
let hardwareDraftEdited = false;
let connectionMissing = false;
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
  document.documentElement.dataset.platform=result.platform||(/Windows/i.test(navigator.platform)?'windows':'macos');
  const wasPermitted=ready();
  runtimeState = result;
  if(result.error&&result.error!==lastRuntimeError)toast(result.error,true);
  lastRuntimeError=result.error||'';
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
  for(const action of ['水平滚动','application','more']){const el=document.querySelector(`[data-action="${action}"]`);el.hidden=result.platform==='windows';}
  renderPermissions();
  updateDraftLabel();
}
async function windowAction(action) {
  if (!invoke) return;
  try { await invoke(action); }
  catch (error) { toast(error.message||String(error),true); }
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
  renderConnectionGate();
}
function setConnectionGate(disconnected) {
  connectionMissing=disconnected;
  renderConnectionGate();
}
function renderConnectionGate() {
  // Permission setup must remain reachable before device access is possible.
  const blocked=connectionMissing&&ready();
  const wasHidden=$('connection-gate').hidden;
  $('connection-gate').hidden=!blocked;
  document.querySelector('.device-header').inert=blocked;
  document.querySelector('.workspace').inert=blocked;
  if(blocked){
    stopRecording();
    if(wasHidden)$('connection-refresh').focus({preventScroll:true});
  }
}
function chooseButton(key) {
  if(key&&!ready())return;
  if(selectedButton!==key)editorMode=null;
  selectedCategory='';
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
  const value=draftConfig[selectedButton];
  chosenAction=editorMode||(ACTIONS.has(value)?value:value?.startsWith('打开应用:')?'application':actionCatalog.categories.some(c=>c.actions?.some(a=>a.value===value))?'more':'shortcut');
  $$('.action-option').forEach(el=>{el.classList.toggle('selected',el.dataset.action===chosenAction);el.setAttribute('aria-pressed',String(el.dataset.action===chosenAction));});
  $('shortcut-editor').hidden=chosenAction!=='shortcut';
  $('more-editor').hidden=chosenAction!=='more';
  $('application-editor').hidden=chosenAction!=='application';
  $('shortcut').value=chosenAction==='shortcut'&&!ACTIONS.has(value)&&!value.startsWith('打开应用:')?value:'';
  const path=value?.startsWith('打开应用:')?value.slice('打开应用:'.length):'';
  $('application-name').textContent=path?path.split('/').pop().replace(/\.app$/i,''):'选择要打开的 App';
  $('application-name').title=path;
  if(chosenAction==='more')renderMoreSelectors(value);
  updateDraftLabel();
}
function chooseAction(value) {
  if(!draftConfig||!selectedButton||!ready())return;
  stopRecording();
  editorMode=value;
  if(!['more','application','shortcut'].includes(value))draftConfig[selectedButton]=value;
  renderAction();
  if(value==='shortcut')$('shortcut').focus();
}
function beginRecording() {
  if(chosenAction!=='shortcut'||!selectedButton||!ready())return;
  recording=true;$('shortcut').classList.add('recording');
  $('shortcut').placeholder='按下组合键';
}
function stopRecording() {
  recording=false;$('shortcut').classList.remove('recording');
  $('shortcut').placeholder='点击后按下组合键';
}
const keyNames={ArrowLeft:'Left',ArrowRight:'Right',ArrowUp:'Up',ArrowDown:'Down',Escape:'Esc',Enter:'Enter',' ':'Space',Delete:'Delete',PageUp:'PageUp',PageDown:'PageDown',Home:'Home',End:'End',Tab:'Tab'};
function record(event) {
  if(!recording||event.target!==$('shortcut')||!selectedButton||!ready())return;
  event.preventDefault();event.stopPropagation();
  if(event.repeat||['Meta','Control','Alt','Shift'].includes(event.key))return;
  if(event.key==='Escape'&&!event.metaKey&&!event.ctrlKey&&!event.altKey&&!event.shiftKey){$('shortcut').blur();return;}
  const key=keyNames[event.key]||(/^Key[A-Z]$/.test(event.code)?event.code.slice(3):/^Digit[0-9]$/.test(event.code)?event.code.slice(5):event.key.toUpperCase());
  const modifiers=[event.metaKey?(runtimeState?.platform==='windows'?'Win':'Cmd'):null,event.ctrlKey?'Ctrl':null,event.altKey?(runtimeState?.platform==='windows'?'Alt':'Option'):null,event.shiftKey?'Shift':null].filter(Boolean);
  const maxFunction=runtimeState?.platform==='windows'?24:20;
  const supported=/^[A-Z0-9]$/.test(key)||(/^F\d+$/.test(key)&&Number(key.slice(1))>=1&&Number(key.slice(1))<=maxFunction)||Object.values(keyNames).includes(key);
  if(!supported){toast(`这个按键暂不支持，请使用字母、数字、方向键或 F1–F${maxFunction}。`,true);return;}
  draftConfig[selectedButton]=[...modifiers,key].join('+');
  renderAction();
}
function renderMoreSelectors(value=draftConfig?.[selectedButton]) {
  const category=$('more-category'),action=$('more-action');
  const current=actionCatalog.categories.find(c=>c.actions.some(a=>a.value===value));
  if(current&&!selectedCategory)selectedCategory=current.id;
  if(!actionCatalog.categories.some(c=>c.id===selectedCategory))selectedCategory=actionCatalog.categories[0]?.id||'';
  category.replaceChildren();
  for(const c of actionCatalog.categories){const o=new Option(c.label,c.id);category.append(o);}
  category.value=selectedCategory;
  const group=actionCatalog.categories.find(c=>c.id===selectedCategory);
  action.replaceChildren(new Option('选择动作',''));
  for(const a of group?.actions||[])action.append(new Option(a.label,a.value));
  action.value=group?.actions.some(a=>a.value===value)?value:'';
  category.disabled=!actionCatalog.categories.length;action.disabled=category.disabled;
}
async function loadActionCatalog() {
  if(runtimeState?.platform==='windows')return;
  try{actionCatalog=await native({action:'actionCatalog'});if(selectedButton)renderAction();}
  catch(error){toast(error.message,true);}
}
async function chooseApplication() {
  if(!selectedButton||!ready())return;
  const button=selectedButton;
  $('choose-application').disabled=true;
  try{
    const result=await native({action:'chooseApplication'});
    if(!result.cancelled&&result.value&&selectedButton===button){draftConfig[button]=result.value;editorMode='application';renderAction();}
  }catch(error){toast(error.message,true);}
  finally{$('choose-application').disabled=false;}
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
  return (actual.capabilities.dpi&&draftHardware.dpi!==actual.dpi)||(actual.capabilities.scroll&&draftHardware.scroll!==actual.scroll?.mode)||(draftHardware.scroll==='smart_shift'&&draftHardware.sensitivity!==actual.scroll?.sensitivity)||(actual.capabilities.scroll_force&&draftHardware.force!==actual.scroll?.force);
}
function renderRefreshControls() {
  for(const id of ['refresh-hardware','refresh-device'])$(id).disabled=hardwareBusy||!ready();
  $('connection-refresh').disabled=hardwareBusy||polling||!ready();
  $('connection-refresh').textContent=hardwareBusy||polling?'正在检测…':'重新检测';
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
  const forceSupported=caps.scroll_force&&runtimeState?.platform!=='windows';
  $('scroll-force-controls').hidden=!forceSupported;
  $('scroll-force-controls').disabled=hardwareBusy||!verified||!forceSupported;
  if (draftHardware) {
    $('dpi-range').value=draftHardware.dpi??1000;$('dpi-value').textContent=draftHardware.dpi??'—';
    $('sensitivity').value=draftHardware.sensitivity??50;$('sensitivity-value').textContent=`${draftHardware.sensitivity??50}%`;
    $('scroll-force').value=draftHardware.force??50;$('scroll-force-value').textContent=`${draftHardware.force??'—'}%`;
    $$('[data-mode]').forEach(el=>{el.classList.toggle('selected',el.dataset.mode===draftHardware.scroll);el.setAttribute('aria-pressed',String(el.dataset.mode===draftHardware.scroll));});
  }
  const dirty=Boolean(hardwareDirty());
  const state=hardwareBusy?'busy':!verified||!ready()?'unavailable':dirty?'pending':'saved';
  const label=hardwareBusy?'同步中…':!ready()?'请先授权':!verified?'设备暂不可用':dirty?'应用到鼠标':'已保存';
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
    if (!oldDirty||!draftHardware) draftHardware={dpi:actual.dpi,scroll:actual.scroll?.mode||'ratchet',sensitivity:actual.scroll?.sensitivity??50,force:actual.scroll?.force??50};
    renderBattery(actual.battery);
  }
  $('connection-dot').className='status-dot'+(verified?'':' error');
  $('connection-text').textContent=verified?'已连接':'设备暂不可用';
  renderHardware();
}
function renderBattery(battery) {
    const levels={full:'满电',good:'电量较高',low:'电量偏低',critical:'电量极低',empty:'电量耗尽'};
    const charging=['recharging','almost_full','slow_recharge'].includes(battery?.status);
    const amount=battery?.percent!=null?`${battery.percent}%`:levels[battery?.level]||'—';
    $('battery').textContent=charging?`充电中 ${amount}`:battery?.status==='full'?`已充满 ${amount}`:`电量 ${amount}`;
    $('battery').classList.toggle('charging',charging);
    $('battery').title=charging?'充电中':battery?.status==='full'?'已充满':'电池电量';
    $('battery').setAttribute('aria-label',`${charging?'充电中':'电量'} ${amount}`);
}
async function refreshBattery() {
  if(batteryBusy||hardwareBusy||!verified||!ready()||runtimeState?.smoke||runtimeState?.platform==='windows')return;
  batteryBusy=true;lastBatteryProbe=Date.now();
  const generation=deviceGeneration;
  try{
    const result=await invoke('hardware',{request:{action:'battery'}});
    if(generation===deviceGeneration&&ready()&&result.ok&&result.target_verified&&actual){
      actual.battery=result.status?.battery??null;
      hardwareResult.status.battery=actual.battery;
      renderBattery(actual.battery);
    }
  }catch(error){/* A failed battery refresh leaves hardware controls and drafts unchanged. */}
  finally{batteryBusy=false;}
}
async function probe(showMessage=false) {
  if (hardwareBusy||runtimeState?.smoke||!invoke||!ready()) return;
  hardwareBusy=true;lastProbe=Date.now();lastBatteryProbe=lastProbe;renderHardware();
  const generation=deviceGeneration;
  try {
    const result=await invoke('hardware',{request:{action:'probe'}});
    if (generation!==deviceGeneration) return;
    updateDeviceStatus(result);
    // A receiver endpoint can remain present while its paired mouse is offline.
    setConnectionGate(!verified);
    if(showMessage)toast(result.ok?'设备状态已更新':result.error||'读取失败，请唤醒鼠标后重试。',!result.ok);
  } catch(error){verified=false;setConnectionGate(true);if(showMessage)toast(String(error),true);}
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
  if(caps.scroll_force&&submitted.force!==actual.scroll?.force)request.scroll_force=submitted.force;
  hardwareBusy=true;renderHardware();
  const generation=deviceGeneration;
  try {
    const result=await invoke('hardware',{request});
    if(generation!==deviceGeneration){toast('连接已改变，请重新确认设备状态。',true);return;}
    updateDeviceStatus(result);
    if(result.ok){hardwareDraftEdited=false;draftHardware={dpi:actual.dpi,scroll:actual.scroll?.mode||'ratchet',sensitivity:actual.scroll?.sensitivity??50,force:actual.scroll?.force??50};toast('已应用，并确认鼠标实际设置');}
    else toast(result.error||'未能确认设置，请重试读取。',true);
  } catch(error){verified=false;toast(String(error),true);}
  finally{hardwareBusy=false;renderHardware();}
}
async function poll(force=false) {
  if(polling||!runtimeState||runtimeState.smoke)return;
  polling=true;
  try {
    const result=await native({action:'state'});renderRuntime(result);
    const metadata=await native({action:'metadata'});
    const targets=metadata.devices.filter(d=>d.hidpp_candidate);
    const next=JSON.stringify(targets.map(d=>d.registry_id).sort());
    if(next!==topology||force){
      topology=next;deviceGeneration++;verified=false;lastProbe=0;
      if(targets.length===0){actual=null;if(!hardwareDraftEdited)draftHardware=null;setConnectionGate(true);$('connection-dot').className='status-dot pending';$('connection-text').textContent='鼠标未连接';renderBattery(null);renderHardware();}
      else {if(!hardwareBusy)await probe();}
    } else if(targets.length>0&&!verified&&!hardwareBusy&&Date.now()-lastProbe>(connectionMissing?8000:30000)&&!document.hidden){await probe();}
    else if(verified&&!document.hidden&&Date.now()-lastBatteryProbe>15000){await refreshBattery();}
  } catch(error){if(force)toast('暂时无法检测鼠标，请稍后重试。',true);}
  finally{polling=false;renderRefreshControls();}
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
  const compactLayout=!document.querySelector('.sidebar,.version,.footer,.button-tabs,.nav-label,#page-buttons .visual-heading')&&(document.documentElement.dataset.platform==='windows'||document.querySelector('.titlebar').textContent.trim()==='');
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
  chooseButton('back');
  const originalBack=draftConfig.back;
  chooseAction('shortcut');
  const focusStartsRecording=recording&&$('shortcut').readOnly&&document.activeElement===$('shortcut')&&!$('record-shortcut');
  const captured=new KeyboardEvent('keydown',{key:'K',code:'KeyK',metaKey:true,shiftKey:true,bubbles:true,cancelable:true});
  $('shortcut').dispatchEvent(captured);
  const isMac=runtimeState.platform!=='windows';
  const shortcutCaptured=draftConfig.back===`${isMac?'Cmd':'Win'}+Shift+K`&&captured.defaultPrevented&&chosenAction==='shortcut';
  $('shortcut').blur();
  const blurStopsRecording=!recording;
  let categoryActionSelected=true,horizontalSelection=true,appPickerAvailable=true;
  if(isMac){
  chooseAction('more');
  $('more-category').value='editing';$('more-category').dispatchEvent(new Event('change'));
  $('more-action').value='Cmd+C';$('more-action').dispatchEvent(new Event('change'));
  categoryActionSelected=chosenAction==='more'&&!$('more-editor').hidden&&draftConfig.back==='Cmd+C'&&mappingDirty();
  chooseAction('水平滚动');
  horizontalSelection=draftConfig.back==='水平滚动'&&$('save-mapping').dataset.state==='pending';
  chooseAction('application');
  appPickerAvailable=!$('application-editor').hidden&&draftConfig.back==='水平滚动';
  }
  draftConfig.back=originalBack;editorMode=null;chooseButton(null);
  updateDeviceStatus({ok:true,target_verified:true,status:{capabilities:{dpi:true,scroll:true,smart_shift:true,scroll_force:true},dpi:1000,dpi_ranges:[{min:200,max:4000,step:50}],scroll:{mode:'smart_shift',sensitivity:50,force:45},battery:{percent:80,status:'recharging'}}});
  draftHardware={dpi:1000,scroll:'smart_shift',sensitivity:50,force:45};hardwareDraftEdited=false;renderHardware();
  $('scroll-force').value='60';$('scroll-force').dispatchEvent(new Event('input'));
  const forceDraftSeparate=isMac?draftHardware.force===60&&actual.scroll.force===45&&hardwareDirty()&&!$('scroll-force-controls').hidden&&$('scroll-force').min==='1':$('scroll-force-controls').hidden;
  const chargingVisible=$('battery').textContent==='充电中 80%'&&$('battery').classList.contains('charging');
  renderBattery({percent:100,status:'full'});
  const fullChargeVisible=$('battery').textContent==='已充满 100%'&&!$('battery').classList.contains('charging');

  // Verify the disconnected layer with the actual WebView hit-testing/layout.
  setConnectionGate(true);
  const gate=$('connection-gate'),gateBounds=gate.getBoundingClientRect();
  const connectionGateCoversContent=!gate.hidden&&Math.abs(gateBounds.top-titlebarHeight)<1&&Math.abs(gateBounds.left)<1&&Math.abs(gateBounds.right-innerWidth)<1&&Math.abs(gateBounds.bottom-innerHeight)<1;
  const connectionGateBlocksBackground=document.querySelector('.device-header').inert&&document.querySelector('.workspace').inert&&document.activeElement===$('connection-refresh');
  const gateCapturesPointer=gate.contains(document.elementFromPoint(innerWidth/2,innerHeight-8));
  const closeControl=$('window-close'),closeRect=closeControl.getBoundingClientRect();
  const closeControlAccessible=document.documentElement.dataset.platform!=='windows'||closeControl.contains(document.elementFromPoint(closeRect.x+closeRect.width/2,closeRect.y+closeRect.height/2));
  const originalPlatform=document.documentElement.dataset.platform;
  document.documentElement.dataset.platform='macos';
  const macWindowControlsHidden=getComputedStyle($('window-controls')).display==='none';
  document.documentElement.dataset.platform=originalPlatform;
  page('settings');
  const connectionGateBlocksSettings=!gate.hidden&&document.querySelector('.workspace').inert;
  fixture.permissions={accessibility:false,inputMonitoring:false,postEvents:false};renderRuntime(fixture);
  const permissionSetupAccessible=gate.hidden&&!document.querySelector('.workspace').inert;
  fixture.permissions={accessibility:true,inputMonitoring:true,postEvents:true};renderRuntime(fixture);
  const gateRestoredAfterPermission=!gate.hidden;
  setConnectionGate(false);
  const connectionRestoresContent=gate.hidden&&!document.querySelector('.device-header').inert&&!document.querySelector('.workspace').inert;
  page('buttons');
  renderRuntime(original);
  const checks={focusStartsRecording,shortcutCaptured,blurStopsRecording,categoryActionSelected,horizontalSelection,appPickerAvailable,forceDraftSeparate,chargingVisible,fullChargeVisible,connectionGateCoversContent,connectionGateBlocksBackground,gateCapturesPointer,closeControlAccessible,macWindowControlsHidden,connectionGateBlocksSettings,permissionSetupAccessible,gateRestoredAfterPermission,connectionRestoresContent,settingsFitsWithoutScroll,permissionGreen,hardwareSavedButton,mappingPendingButton,closeGlyphCentered,partialPermissionActions,grantedPermissionActionsHidden,wheelCalloutFits,sliderDraftSeparate,staleHardwareHidden,mouseCentered,hardwareOpensFromMouse,wheelHotspotAligned,hardwareDismissed,permissionPromptInline,appIconLoaded:runtimeState.appIconLoaded===true,compactLayout,balancedInsets,statusCapsuleFits,mappingStatusSynced,mappingSettingsAccessible,permissionGateBlocks,settingsAccessible,settingsScrollbarHidden,partialPermissionBlocks,permissionUnlocks,revocationBlocks,initialUnselected,selectionWorks,panelAnimated,hotspotAligned,navigationWorks,draftPreserved,editorDismissed,scrollbarHidden,mouseImageLoaded:image.naturalWidth>0};
  const report={ok:Object.values(checks).every(Boolean),...checks,windowVisible:true,frontendLoaded:true,mappingActive:runtimeState.active,configWritten:false,hardwareWrites:false,permissionRequested:false};
  await invoke('smoke_report',{report});
}
$$('[data-page]').forEach(el=>el.addEventListener('click',()=>page(el.dataset.page)));
$$('[data-button]').forEach(el=>el.addEventListener('click',()=>chooseButton(el.dataset.button)));
$$('[data-action]').forEach(el=>el.addEventListener('click',()=>chooseAction(el.dataset.action)));
$('shortcut').addEventListener('focus',beginRecording);
$('shortcut').addEventListener('click',beginRecording);
$('shortcut').addEventListener('blur',stopRecording);
$('more-category').addEventListener('change',()=>{selectedCategory=$('more-category').value;renderMoreSelectors();});
$('more-action').addEventListener('change',()=>{if(selectedButton&&$('more-action').value){draftConfig[selectedButton]=$('more-action').value;renderAction();}});
document.addEventListener('keydown',record,true);
$('close-editor').addEventListener('click',()=>chooseButton(null));
$('close-hardware').addEventListener('click',()=>chooseButton(null));
$('wheel-hotspot').addEventListener('click',openHardware);
$('settings-toggle').addEventListener('click',()=>page(currentPage==='settings'?'buttons':'settings'));
$('connection-refresh').addEventListener('click',()=>poll(true));
$('window-minimize').addEventListener('click',()=>windowAction('window_minimize'));
$('window-maximize').addEventListener('click',()=>windowAction('window_toggle_maximize'));
$('window-close').addEventListener('click',()=>windowAction('window_close'));
$('save-mapping').addEventListener('click',saveMapping);$('mapping-toggle').addEventListener('change',toggleMapping);
$('dpi-range').addEventListener('input',()=>{if(!draftHardware)return;draftHardware.dpi=Number($('dpi-range').value);hardwareDraftEdited=hardwareDirty();renderHardware();});
$$('[data-mode]').forEach(el=>el.addEventListener('click',()=>{if(!draftHardware)return;draftHardware.scroll=el.dataset.mode;hardwareDraftEdited=hardwareDirty();renderHardware();}));
$('sensitivity').addEventListener('input',()=>{if(!draftHardware)return;draftHardware.sensitivity=Number($('sensitivity').value);hardwareDraftEdited=hardwareDirty();renderHardware();});
$('scroll-force').addEventListener('input',()=>{if(!draftHardware)return;draftHardware.force=Number($('scroll-force').value);hardwareDraftEdited=hardwareDirty();renderHardware();});
$('choose-application').addEventListener('click',chooseApplication);
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
    await loadActionCatalog();
    if(result.smoke){await smokeCheck();return;}
    await poll();setInterval(poll,4000);
  } catch(error){toast(error.message,true);}
}
start();
