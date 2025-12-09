<template>
  <div
    class="typebuddy-card"
    :class="{ 'typebuddy-card--english': !isZh }"
  >
    <a
      href="https://typebuddy.ai/"
      target="_blank"
      rel="noopener noreferrer"
      class="typebuddy-card__link"
    >
      <div class="typebuddy-card__header">
        <img
          src="https://img.typebuddy.ai/logo.png"
          alt="TypeBuddy"
          class="typebuddy-card__logo"
        />
        <div class="typebuddy-card__brand">TypeBuddy</div>
      </div>

      <div class="typebuddy-card__content">
        <h3 class="typebuddy-card__tagline">{{ t.tagline }}</h3>
        <p class="typebuddy-card__description">{{ t.description }}</p>
      </div>

      <div class="typebuddy-card__footer">
        <span class="typebuddy-card__cta">{{ t.cta }}</span>
      </div>
    </a>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useData } from 'vitepress'

const { lang } = useData()

const isZh = computed(
  () => lang.value === 'zh-Hans' || lang.value.startsWith('zh')
)

const t = computed(() => {
  return isZh.value
    ? {
        tagline: '本地离线模型驱动的 AI 输入助手',
        description: '毫秒级实时补全/改写/翻译/语法检查一站搞定！',
        cta: '立即体验 →',
      }
    : {
        tagline: 'Local Offline Model-Powered AI Input Assistant',
        description:
          'Millisecond Real-time Completion/Rewriting/Translation/Grammar Check All-in-One!',
        cta: 'Try Now →',
      }
})
</script>

<style scoped>
.typebuddy-card {
  width: 224px;
  height: 224px;
  background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 8px 25px rgba(59, 130, 246, 0.15);
  transition: all 0.3s ease;
  position: relative;
}

.typebuddy-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0) 100%
  );
  pointer-events: none;
}

.typebuddy-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 12px 35px rgba(59, 130, 246, 0.25);
}

.typebuddy-card__link {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 18px;
  text-decoration: none;
  color: white;
  position: relative;
  z-index: 1;
}

.typebuddy-card__header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.typebuddy-card__logo {
  width: 24px;
  height: 24px;
  object-fit: contain;
}

.typebuddy-card__brand {
  font-size: 16px;
  font-weight: 800;
  color: white;
  letter-spacing: -0.5px;
}

.typebuddy-card__content {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.typebuddy-card__tagline {
  font-size: 14px;
  font-weight: 700;
  margin: 0 0 10px 0;
  color: #ffffff;
  line-height: 1.3;
}

.typebuddy-card__description {
  font-size: 12px;
  margin: 0 0 16px 0;
  color: rgba(255, 255, 255, 0.9);
  line-height: 1.4;
}

.typebuddy-card__footer {
  text-align: center;
  margin-top: auto;
}

.typebuddy-card__cta {
  display: inline-block;
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(10px);
  padding: 8px 16px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  color: white;
  border: 1px solid rgba(255, 255, 255, 0.2);
  transition: all 0.2s ease;
}

.typebuddy-card__link:hover .typebuddy-card__cta {
  background: rgba(255, 255, 255, 0.25);
  transform: scale(1.05);
}

/* Dark mode adjustments */
.dark .typebuddy-card {
  background: linear-gradient(135deg, #1e40af 0%, #1e3a8a 100%);
  box-shadow: 0 8px 25px rgba(30, 64, 175, 0.2);
}

.dark .typebuddy-card:hover {
  box-shadow: 0 12px 35px rgba(30, 64, 175, 0.3);
}

/* English version gets more height for longer text */
.typebuddy-card.typebuddy-card--english {
  height: 240px;
}
</style>
