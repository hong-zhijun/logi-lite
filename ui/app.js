'use strict';
const $ = id => document.getElementById(id);
const $$ = selector => [...document.querySelectorAll(selector)];
const invoke = window.__TAURI__?.core.invoke;
const BUTTONS = {back:'后退侧键',forward:'前进侧键',middle:'中键（鼠标滚轮）',top:'顶部按钮'};
const ACTIONS = new Set(['默认','禁用','打开 App','调度中心']);
let actionCatalog = {categories:[]}, editorMode=null;
let selectedCategory='',openSelect=null;
const applicationCache=new Map();
let pageTransition=0;
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
function normalizeConfig(config) {
  const value=clone(config);
  value.horizontalScroll={back:false,forward:false,...value.horizontalScroll};
  value.top??='默认';
  return value;
}
const mappingDirty = () => draftConfig && savedConfig && (Object.keys(BUTTONS).some(key=>draftConfig[key]!==savedConfig[key])||['back','forward'].some(key=>Boolean(draftConfig.horizontalScroll?.[key])!==Boolean(savedConfig.horizontalScroll?.[key])));
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
  if (initial || !draftConfig) {savedConfig=normalizeConfig(result.config);draftConfig=normalizeConfig(result.config);renderAction();}
  else {savedConfig=normalizeConfig(result.config);}
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
  for(const action of ['application','more']){const el=document.querySelector(`[data-action="${action}"]`);el.hidden=result.platform==='windows';}
  renderPermissions();
  renderTopStatus();
  updateDraftLabel();
}
function renderTopStatus() {
  const message=runtimeState?.topButtonStatus||'';
  $('top-button-status').textContent=message;
  $('top-button-status').hidden=selectedButton!=='top'||!message;
}
async function windowAction(action) {
  if (!invoke) return;
  try { await invoke(action); }
  catch (error) { toast(error.message||String(error),true); }
}
function pageIsActive(name) {return currentPage===name;}
function page(name) {
  closeSelect();
  const previous=currentPage;
  currentPage=name;
  const generation=++pageTransition;
  const animated=previous!==name&&!matchMedia('(prefers-reduced-motion: reduce)').matches;
  for(const el of $$('.page')) {
    el.getAnimations().forEach(animation=>animation.cancel());
    const active=el.id===`page-${name}`;
    el.setAttribute('aria-hidden',String(!active));
    el.inert=!active;
    if(active){
      el.hidden=false;
      if(animated)el.animate([{opacity:0,transform:'translateY(10px)'},{opacity:1,transform:'translateY(0)'}],{duration:260,easing:'cubic-bezier(.22,1,.36,1)'});
    } else if(animated&&!el.hidden){
      el.animate([{opacity:1,transform:'translateY(0)'},{opacity:0,transform:'translateY(-6px)'}],{duration:160,easing:'ease-out',fill:'forwards'}).finished.then(()=>{if(generation===pageTransition){el.hidden=true;el.getAnimations().forEach(animation=>animation.cancel());}}).catch(()=>{});
    } else el.hidden=true;
  }
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
  $('page-buttons').inert=!permitted||currentPage!=='buttons';
  $('mapping-toggle').disabled=!permitted;
  $('gate-authorize').disabled=!runtimeState;
  renderRefreshControls();
  for(const [id,label,granted] of [['gate-accessibility','辅助功能',runtimeState?.permissions.accessibility],['gate-input','输入监控',runtimeState?.permissions.inputMonitoring]]) {
    $(id).textContent=`${label} · ${runtimeState?(granted?'已授权':'待授权'):'检查中'}`;
    $(id).classList.toggle('ready',Boolean(granted));
  }
  if(!permitted){$('connection-text').textContent=runtimeState?'等待授权':'正在检查权限…';$('connection-dot').className='status-dot pending';}
  if(gated){stopRecording();closeSelect();}
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
    closeSelect();stopRecording();
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
  closeSelect();
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
  renderTopStatus();
  if (!draftConfig || !selectedButton || !ready()) return;
  const windows=runtimeState?.platform==='windows';
  for(const action of ['打开 App','调度中心','application','more']){
    const option=document.querySelector(`[data-action="${action}"]`);
    if(option)option.hidden=windows;
  }
  const value=draftConfig[selectedButton];
  document.querySelector('[data-action="默认"] span').textContent=selectedButton==='top'?'切换滚轮模式':'默认';
  document.querySelector('[data-action="默认"] use').setAttribute('href',selectedButton==='top'?'#i-refresh':'#i-arrow');
  chosenAction=editorMode||(ACTIONS.has(value)?value:value?.startsWith('打开应用:')?'application':actionCatalog.categories.some(c=>c.actions?.some(a=>a.value===value))?'more':'shortcut');
  $$('.action-option').forEach(el=>{el.classList.toggle('selected',el.dataset.action===chosenAction);el.setAttribute('aria-pressed',String(el.dataset.action===chosenAction));});
  $('shortcut-editor').hidden=chosenAction!=='shortcut';
  $('more-editor').hidden=chosenAction!=='more';
  $('application-editor').hidden=chosenAction!=='application';
  $('shortcut').value=chosenAction==='shortcut'&&!ACTIONS.has(value)&&!value.startsWith('打开应用:')?value:'';
  renderApplication(value);
  $('horizontal-scroll-row').hidden=!['back','forward'].includes(selectedButton);
  $('horizontal-scroll-toggle').checked=Boolean(draftConfig.horizontalScroll?.[selectedButton]);
  if(chosenAction==='more')renderMoreSelectors(value);
  updateDraftLabel();
}
function chooseAction(value) {
  closeSelect();
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
const categoryIcons={system:'grid',editing:'key',windows:'windows',browser:'windows',media:'sliders'};
function renderMoreSelectors(value=draftConfig?.[selectedButton]) {
  const current=actionCatalog.categories.find(c=>c.actions.some(a=>a.value===value));
  if(current&&!selectedCategory)selectedCategory=current.id;
  if(!actionCatalog.categories.some(c=>c.id===selectedCategory))selectedCategory=actionCatalog.categories[0]?.id||'';
  const group=actionCatalog.categories.find(c=>c.id===selectedCategory);
  const action=group?.actions.find(a=>a.value===value);
  $('more-category').querySelector('.select-value').textContent=group?.label||'选择类别';
  $('more-category').querySelector('.select-icon use').setAttribute('href',`#i-${categoryIcons[group?.id]||'grid'}`);
  $('more-action').querySelector('.select-value').textContent=action?.label||'选择动作';
  $('more-action').classList.toggle('placeholder',!action);
  $('more-category').disabled=!group;
  $('more-action').disabled=!group;
}
function closeSelect(restoreFocus=false) {
  const trigger=openSelect?$(openSelect):null;
  openSelect=null;
  $('select-popover').hidden=true;
  for(const id of ['more-category','more-action'])$(id).setAttribute('aria-expanded','false');
  if(restoreFocus)trigger?.focus({preventScroll:true});
}
function openSelectMenu(id,focusLast=false) {
  if(openSelect===id){closeSelect(true);return;}
  closeSelect();
  const trigger=$(id),menu=$('select-popover');
  if(trigger.disabled||!selectedButton||!ready())return;
  const category=id==='more-category';
  const group=actionCatalog.categories.find(c=>c.id===selectedCategory);
  const options=category?actionCatalog.categories:(group?.actions||[]);
  const selected=category?selectedCategory:draftConfig[selectedButton];
  menu.replaceChildren();
  for(const option of options){
    const button=document.createElement('button');
    button.type='button';button.className='select-option';button.setAttribute('role','option');
    const checked=(category?option.id:option.value)===selected;
    button.setAttribute('aria-selected',String(checked));
    const label=document.createElement('span');label.textContent=option.label;button.append(label);
    const check=document.createElementNS('http://www.w3.org/2000/svg','svg');check.setAttribute('viewBox','0 0 24 24');check.setAttribute('aria-hidden','true');
    const use=document.createElementNS('http://www.w3.org/2000/svg','use');use.setAttribute('href','#i-check');check.append(use);button.append(check);
    button.addEventListener('click',()=>{
      if(category)selectedCategory=option.id;
      else draftConfig[selectedButton]=option.value;
      closeSelect();renderAction();trigger.focus({preventScroll:true});
    });
    menu.append(button);
  }
  openSelect=id;menu.hidden=false;trigger.setAttribute('aria-expanded','true');
  menu.setAttribute('aria-label',category?'动作类别':'具体动作');
  const rect=trigger.getBoundingClientRect(),desired=Math.min(284,menu.scrollHeight),below=innerHeight-rect.bottom-12;
  const above=below<Math.min(desired,180)&&rect.top>below;
  const height=Math.min(desired,above?rect.top-12:below);
  menu.style.width=`${rect.width}px`;menu.style.left=`${rect.left}px`;
  menu.style.maxHeight=`${height}px`;menu.style.top=`${above?rect.top-height-6:rect.bottom+6}px`;
  const buttons=[...menu.children];
  (focusLast?buttons.at(-1):buttons.find(b=>b.getAttribute('aria-selected')==='true')||buttons[0])?.focus({preventScroll:true});
}
function renderApplication(value) {
  const path=value?.startsWith('打开应用:')?value.slice('打开应用:'.length):'';
  const info=applicationCache.get(path);
  const metadata=info&&!info.then?info:null;
  const name=metadata?.name||path.split('/').pop()?.replace(/\.app$/i,'')||'选择要打开的 App';
  const icon=metadata?.icon?.startsWith('data:image/png;base64,')?metadata.icon:null;
  $('application-name').textContent=name;
  $('application-name').title=path;
  $('application-picker-label').textContent=path?(metadata?.available===false?'应用不可用 · 更换应用':'更换应用'):'选择应用…';
  $('application-action-label').textContent=path?name:'打开指定 App';
  const tile=document.querySelector('[data-action="application"]');
  tile.title=path?`打开 ${name}`:'打开指定 App';
  tile.setAttribute('aria-label',path?`打开指定 App：${name}`:'打开指定 App');
  for(const id of ['application-image','application-tile-image']) {
    const img=$(id);img.hidden=!icon;img.previousElementSibling.toggleAttribute('hidden',Boolean(icon));
    if(icon)img.src=icon;else img.removeAttribute('src');
  }
  if(path&&!info){
    const request=native({action:'applicationInfo',path});
    applicationCache.set(path,request);
    request.then(result=>{applicationCache.set(path,result);if(draftConfig?.[selectedButton]===value)renderApplication(value);})
      .catch(()=>{applicationCache.set(path,{name,available:false});if(draftConfig?.[selectedButton]===value)renderApplication(value);});
  }
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
    if(!result.cancelled&&result.value&&selectedButton===button){applicationCache.set(result.path||result.value.slice('打开应用:'.length),result);draftConfig[button]=result.value;editorMode='application';renderAction();}
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
  const forceSupported=Boolean(caps.scroll_force);
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
  const settingsAccessible=pageIsActive('settings')&&!$('page-settings').inert&&$('permission-gate').hidden;
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
  const mappingSettingsAccessible=pageIsActive('settings')&&$('page-settings').contains($('mapping-toggle'));
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
  const hardwareOpensFromMouse=selectedPanel==='hardware'&&!$('hardware-editor').inert&&$('button-editor').inert&&pageIsActive('buttons');
  const wheel=document.querySelector('#wheel-hotspot').getBoundingClientRect();
  const wheelHotspotAligned=Math.abs((wheel.x+wheel.width/2-r.x)/r.width-.60)<.015&&Math.abs((wheel.y+wheel.height/2-r.y)/r.height-.12)<.015;
  const callout=$('wheel-hotspot').querySelector('.hotspot-label').getBoundingClientRect();
  const middle=document.querySelector('[data-button="middle"] .hotspot-label').getBoundingClientRect();
  const visual=document.querySelector('.visual-area').getBoundingClientRect();
  const wheelCalloutFits=callout.left>=visual.left&&callout.right<=visual.right&&callout.top>=visual.top&&callout.bottom<=visual.bottom&&callout.top<middle.top;
  updateDeviceStatus({ok:true,target_verified:true,status:{capabilities:{dpi:true,scroll:true,smart_shift:true,scroll_force:true},dpi:1000,dpi_ranges:[{min:200,max:4000,step:50}],scroll:{mode:'smart_shift',sensitivity:50,force:45}}});
  const hardwareSavedButton=$('apply-hardware').dataset.state==='saved'&&$('apply-hardware').disabled;
  $('dpi-range').value='1500';$('dpi-range').dispatchEvent(new Event('input'));
  const sliderDraftSeparate=$('dpi-value').textContent==='1500'&&draftHardware.dpi===1500&&$('actual-dpi').textContent==='DPI 1000'&&$('actual-scroll').textContent==='SmartShift'&&!$('apply-hardware').disabled&&$('apply-hardware').dataset.state==='pending'&&!document.querySelector('#dpi-number,#hardware-editor #actual-hardware');
  updateDeviceStatus({ok:false,error:'设备已断开'});
  const staleHardwareHidden=$('actual-dpi').textContent==='DPI —'&&$('actual-scroll').textContent==='滚轮状态未知'&&$('dpi-controls').disabled;
  $('close-hardware').click();
  const hardwareDismissed=selectedPanel===null&&$('hardware-editor').inert;
  $('settings-toggle').click();
  const navigationWorks=pageIsActive('settings')&&$('page-buttons').inert&&$('settings-toggle').getAttribute('aria-expanded')==='true';
  const settingsAnimated=!motionAllowed||$('page-settings').getAnimations().length>0;
  page('buttons');page('settings');
  const rapidPageSwitchSafe=pageIsActive('settings')&&!$('page-settings').inert&&$('page-buttons').inert;
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
  let categoryActionSelected=true,horizontalSelection=true,appPickerAvailable=true,applicationIconVisible=true,middleHasNoHorizontal=true,sideSwitchesIndependent=true,customSelectStyled=true,topButtonIndependent=true,topDefaultMode=true,topHasNoHorizontal=true,topHotspotAligned=true,topCalloutFits=true,legacyTopDefault=true,topDeviceStatus=true;
  const originalHorizontal=clone(draftConfig.horizontalScroll||{back:false,forward:false});
  if(isMac){
    chooseAction('more');
    $('more-category').click();
    const selectOption=label=>[...$('select-popover').children].find(el=>el.textContent===label)?.click();
    selectOption('编辑');
    $('more-action').click();selectOption('复制');
    categoryActionSelected=chosenAction==='more'&&!$('more-editor').hidden&&draftConfig.back==='Cmd+C'&&mappingDirty();
    customSelectStyled=!document.querySelector('#more-editor select')&&$('more-category').getAttribute('role')==='combobox'&&$('select-popover').hidden;
    const previousAction=draftConfig.back;
    $('horizontal-scroll-toggle').checked=true;$('horizontal-scroll-toggle').dispatchEvent(new Event('change'));
    horizontalSelection=draftConfig.horizontalScroll.back===true&&draftConfig.back===previousAction&&!document.querySelector('[data-action="水平滚动"]');
    chooseButton('forward');
    sideSwitchesIndependent=$('horizontal-scroll-toggle').checked===Boolean(originalHorizontal.forward);
    chooseButton('middle');middleHasNoHorizontal=$('horizontal-scroll-row').hidden;
    const originalTop=draftConfig.top,originalMiddle=draftConfig.middle;
    const legacyConfig=clone(draftConfig);delete legacyConfig.top;
    legacyTopDefault=normalizeConfig(legacyConfig).top==='默认';
    document.querySelector('[data-button="top"]').click();
    topHasNoHorizontal=$('horizontal-scroll-row').hidden;
    chooseAction('默认');
    topDefaultMode=selectedButton==='top'&&$('selected-title').textContent==='顶部按钮'&&document.querySelector('[data-action="默认"] span').textContent==='切换滚轮模式';
    chooseAction('禁用');
    topButtonIndependent=draftConfig.top==='禁用'&&draftConfig.middle===originalMiddle&&mappingDirty()&&draftConfig.horizontalScroll.back===true;
    const topImage=image.getBoundingClientRect(),top=document.querySelector('[data-button="top"]').getBoundingClientRect(),topLabel=document.querySelector('[data-button="top"] .hotspot-label').getBoundingClientRect();
    topHotspotAligned=Math.abs((top.x+top.width/2-topImage.x)/topImage.width-.72)<.015&&Math.abs((top.y+top.height/2-topImage.y)/topImage.height-.34)<.015;
    topCalloutFits=topLabel.left>=visual.left&&topLabel.right<=visual.right&&topLabel.top>middle.bottom&&topLabel.bottom<=visual.bottom;
    const originalTopStatus=runtimeState.topButtonStatus;
    runtimeState.topButtonStatus='顶部按钮等待鼠标连接';renderTopStatus();
    topDeviceStatus=!$('top-button-status').hidden&&$('top-button-status').textContent==='顶部按钮等待鼠标连接';
    chooseButton('middle');
    topDeviceStatus=topDeviceStatus&&$('top-button-status').hidden;
    runtimeState.topButtonStatus=originalTopStatus;
    draftConfig.top=originalTop;
    chooseButton('back');chooseAction('application');
    appPickerAvailable=!$('application-editor').hidden&&draftConfig.back===previousAction;
    const app=await native({action:'applicationInfo',path:'/System/Applications/Calculator.app'});
    draftConfig.back=app.value;applicationCache.set(app.path,app);renderAction();
    await $('application-image').decode();
    applicationIconVisible=app.available&&app.icon.startsWith('data:image/png;base64,')&&!$('application-image').hidden&&!$('application-tile-image').hidden&&$('application-image').naturalWidth>0;
  }
  draftConfig.horizontalScroll=originalHorizontal;
  draftConfig.back=originalBack;editorMode=null;chooseButton(null);
  updateDeviceStatus({ok:true,target_verified:true,status:{capabilities:{dpi:true,scroll:true,smart_shift:true,scroll_force:true},dpi:1000,dpi_ranges:[{min:200,max:4000,step:50}],scroll:{mode:'smart_shift',sensitivity:50,force:45},battery:{percent:80,status:'recharging'}}});
  draftHardware={dpi:1000,scroll:'smart_shift',sensitivity:50,force:45};hardwareDraftEdited=false;renderHardware();
  $('scroll-force').value='60';$('scroll-force').dispatchEvent(new Event('input'));
  const forceDraftSeparate=draftHardware.force===60&&actual.scroll.force===45&&hardwareDirty()&&!$('scroll-force-controls').hidden&&$('scroll-force').min==='1';
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
  const checks={topDeviceStatus,topButtonIndependent,topDefaultMode,topHasNoHorizontal,topHotspotAligned,topCalloutFits,legacyTopDefault,settingsAnimated,rapidPageSwitchSafe,applicationIconVisible,middleHasNoHorizontal,sideSwitchesIndependent,customSelectStyled,focusStartsRecording,shortcutCaptured,blurStopsRecording,categoryActionSelected,horizontalSelection,appPickerAvailable,forceDraftSeparate,chargingVisible,fullChargeVisible,connectionGateCoversContent,connectionGateBlocksBackground,gateCapturesPointer,closeControlAccessible,macWindowControlsHidden,connectionGateBlocksSettings,permissionSetupAccessible,gateRestoredAfterPermission,connectionRestoresContent,settingsFitsWithoutScroll,permissionGreen,hardwareSavedButton,mappingPendingButton,closeGlyphCentered,partialPermissionActions,grantedPermissionActionsHidden,wheelCalloutFits,sliderDraftSeparate,staleHardwareHidden,mouseCentered,hardwareOpensFromMouse,wheelHotspotAligned,hardwareDismissed,permissionPromptInline,appIconLoaded:runtimeState.appIconLoaded===true,compactLayout,balancedInsets,statusCapsuleFits,mappingStatusSynced,mappingSettingsAccessible,permissionGateBlocks,settingsAccessible,settingsScrollbarHidden,partialPermissionBlocks,permissionUnlocks,revocationBlocks,initialUnselected,selectionWorks,panelAnimated,hotspotAligned,navigationWorks,draftPreserved,editorDismissed,scrollbarHidden,mouseImageLoaded:image.naturalWidth>0};
  const report={ok:Object.values(checks).every(Boolean),...checks,windowVisible:true,frontendLoaded:true,mappingActive:runtimeState.active,configWritten:false,hardwareWrites:false,permissionRequested:false};
  await invoke('smoke_report',{report});
}
$$('[data-page]').forEach(el=>el.addEventListener('click',()=>page(el.dataset.page)));
$$('[data-button]').forEach(el=>el.addEventListener('click',()=>chooseButton(el.dataset.button)));
$$('[data-action]').forEach(el=>el.addEventListener('click',()=>chooseAction(el.dataset.action)));
$('shortcut').addEventListener('focus',beginRecording);
$('shortcut').addEventListener('click',beginRecording);
$('shortcut').addEventListener('blur',stopRecording);
for(const id of ['more-category','more-action']){
  $(id).addEventListener('click',()=>openSelectMenu(id));
  $(id).addEventListener('keydown',event=>{if(['ArrowDown','ArrowUp'].includes(event.key)){event.preventDefault();openSelectMenu(id,event.key==='ArrowUp');}});
}
$('select-popover').addEventListener('keydown',event=>{
  const buttons=[...$('select-popover').children],index=buttons.indexOf(document.activeElement);
  if(event.key==='Escape'){event.preventDefault();closeSelect(true);}
  else if(event.key==='Tab')closeSelect();
  else if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){
    event.preventDefault();
    const next=event.key==='Home'?0:event.key==='End'?buttons.length-1:(index+(event.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length;
    buttons[next]?.focus({preventScroll:true});
  }
});
document.addEventListener('pointerdown',event=>{if(openSelect&&!$('select-popover').contains(event.target)&&!$(openSelect).contains(event.target))closeSelect();});
window.addEventListener('resize',()=>closeSelect());
$('button-editor').addEventListener('scroll',()=>closeSelect());
$('horizontal-scroll-toggle').addEventListener('change',()=>{
  if(!ready()||!['back','forward'].includes(selectedButton))return;
  draftConfig.horizontalScroll={back:false,forward:false,...draftConfig.horizontalScroll,[selectedButton]:$('horizontal-scroll-toggle').checked};
  updateDraftLabel();
});
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
