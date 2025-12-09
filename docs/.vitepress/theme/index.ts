import { h } from 'vue'
import DefaultTheme from 'vitepress/theme'
import 'virtual:group-icons.css'
import './styles.css'
import TypeBuddyAd from './components/TypeBuddyAd.vue'

export default {
  extends: DefaultTheme,

  Layout() {
    return h(DefaultTheme.Layout, null, {
      'aside-ads-before': () => h(TypeBuddyAd),
    })
  },
}
