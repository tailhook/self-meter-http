import {createStore, applyMiddleware, compose} from 'redux'
import {attach} from 'khufu-runtime'
import {Router} from 'khufu-routing'

import {main} from './main'

let router = new Router(window);
let khufu = attach(document.getElementById('app'), main(router), {
    store(reducer, middleware, state) {
        return createStore(reducer, state, applyMiddleware(...middleware))
    }
})
router.subscribe(khufu.queue_render)

if(module.hot) {
    module.hot.accept()
}
