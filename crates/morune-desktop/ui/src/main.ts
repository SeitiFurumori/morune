import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import MenuBandeja from './MenuBandeja.svelte'
import { iniciar } from './lib/motor.svelte'

iniciar()
// A mesma pagina serve a janela principal e o menu da bandeja.
const raiz = location.hash === '#bandeja' ? MenuBandeja : App
mount(raiz, { target: document.getElementById('app')! })
