import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { iniciar } from './lib/motor.svelte'

iniciar()
mount(App, { target: document.getElementById('app')! })
