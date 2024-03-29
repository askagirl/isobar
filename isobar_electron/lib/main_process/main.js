const {app, BrowserWindow} = require('electron');
const {spawn} = require('child_process');
const path = require('path');
const url = require('url')
const IsobarClient = require('../shared/isobar_client');

const SERVER_PATH = process.env.ISOBAR_SERVER_PATH;
if (!SERVER_PATH) {
  console.error('Missing ISOBAR_SERVER_PATH environment variable');
  process.exit(1);
}

const SOCKET_PATH = process.env.ISOBAR_SOCKET_PATH;
if (!SOCKET_PATH) {
  console.error('Missing ISOBAR_SOCKET_PATH environment variable');
  process.exit(1);
}

class IsobarApplication {
  constructor (serverPath, socketPath) {
    this.serverPath = serverPath;
    this.socketPath = socketPath;
    this.windowsById = new Map();
    this.readyPromise = new Promise(resolve => app.on('ready', resolve));
    this.isobarClient = new IsobarClient();
  }

  async  start () {
    const serverProcess = spawn(this.serverPath, [], {stdio: ['ignore', 'pipe', 'inherit']});
    app.on('before-quit', () => serverProcess.kill());

    serverProcess.on('error', console.error);
    serverProcess.on('exit', () => app.quit());

    await new Promise(resolve => {
      let serverStdout = '';
      serverProcess.stdout.on('data', data => {
        serverStdout += data.toString('utf8');
        if (serverStdout.includes('Listening\n')) resolve()
      });
    });

    await this.isobarClient.start(this.socketPath);
    this.isobarClient.addMessageListener(this._handleMessage.bind(this));
    this.isobarClient.sendMessage({type: 'StartApp'});
  }

  async _handleMessage (message) {
    await this.readyPromise;
    switch (message.type) {
      case 'OpenWindow': {
        this._createWindow(message.window_id);
        break;
      }
    }
  }

  _createWindow (windowId) {
    const window = new BrowserWindow({width: 800, height: 600, webSecurity: false});
    window.loadURL(url.format({
      pathname: path.join(__dirname, '../../index.html'),
      search: `windowId=${windowId}&socketPath=${encodeURIComponent(this.socketPath)}`,
      protocol: 'file:',
      slashes: true
    }));
    this.windowsById.set(windowId, window);
    window.on('closed', () => {
      this.windowsById.delete(windowId);
      this.isobarClient.sendMessage({type: 'CloseWindow', window_id: windowId});
    })
  }
}

app.commandLine.appendSwitch("enable-experimental-web-platform-features");

app.on('window-all-closed', function () {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

const application = new IsobarApplication(SERVER_PATH, SOCKET_PATH);
application.start().then(() => {
  console.log('Listening');
});
