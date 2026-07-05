/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { classifyIntent } from '../index.ts';

const noneCases = [
  'random thought about roadmap naming',
  'thought about partner pricing names',
  'thought on onboarding copy',
  'random thought on workspace permissions',
  'just a thought about color tokens',
  'a thought regarding weekly planning docs',
  'idea about launch copy',
  'idea for project codename options',
  'just an idea about team naming',
  'idea on the homepage headline',
  'question about roadmap ownership',
  'just a question about pricing tiers',
  'question regarding support coverage',
  'question on naming the analytics tab',
  'question about pilot rollout scope',
  'notes about customer interview themes',
  'some notes on the onboarding flow',
  'notes regarding design critiques',
  'note about q3 messaging',
  'observation about error states',
  'observation on mobile nav labels',
  'reflection on weekly planning habits',
  'musing about icon consistency',
  'random thought regarding beta access',
  'idea regarding plan naming',
  'question for finance policy',
  'notes for next docs pass',
  'reflection regarding roadmap themes',
  'musing on search relevance',
  'just an observation about team rituals',
  'just notes about experimentation',
  'a question regarding copy tone',
  'an idea for a better release codename',
  'a random thought about the partner page',
  'random thought on permission naming',
  'just a thought about help center IA',
  'just an idea on referral messaging',
  'just a question on feature packaging',
  'some notes about invoice language',
  'note regarding app store screenshots',
  'observation regarding editor affordances',
  'reflection about weekly check-in prompts',
  'musing regarding settings IA',
  'question about whether the free tier is too generous',
  'idea about renaming quick actions',
  'notes on internal launch themes',
  'thought about docs taxonomy',
  'question regarding metrics naming',
  'check out Todds movie at some point',
  'consider adding blocks to the website',
  'think about adding blocks to the website',
  'explore adding blocks to the website',
  'auto share the graphs',
  'ability to have a a video cal',
  'random thought about roadmap naming for enterprise',
  'dentist at 4',
] as const;

const taskCases = [
  'buy groceries',
  'call mom',
  'finish board deck tomorrow',
  'fix login bug urgent',
  'send recap email after standup',
  'update pricing page copy',
  'draft partnership proposal',
  'review pull request from maya',
  'prepare q2 forecast',
  'organize receipts',
  'renew passport',
  'submit expense report friday',
  'book dentist cleaning',
  'clean up analytics dashboard',
  'refactor auth middleware',
  'plan launch checklist',
  'email jordan the brief',
  'check warehouse inventory',
  'sign vendor contract',
  'pick up the package tomorrow',
  'add blocks to the website',
  'share the graphs',
] as const;

const calendarCases = [
  'meeting with sarah at 3pm',
  'birthday party saturday',
  'doctor appointment on friday at 4pm',
  'lunch with alex tomorrow at noon',
  'coffee with nina on tuesday at 9',
  'rehearsal thursday 7pm',
  'yoga class saturday at 10am',
  'flight to chicago monday at 6am',
  'offsite next friday at 1pm',
  'town hall tomorrow at 11',
  'interview with candidate at 2pm',
  'dinner reservation friday at 8',
  'product workshop wednesday at 1pm',
  'haircut appointment tuesday at 5pm',
  'brunch with family sunday at 11',
] as const;

const reminderCases = [
  'remind me to call mom',
  'alert me in 10 minutes to take the cake out',
  'tell me at 9pm to wind down',
  'set a reminder to pay rent on friday',
  'reminder to stretch in 30 minutes',
  'remind me about the investor deck tomorrow morning',
  'notify me at noon to send the invoice',
  'create a reminder to water plants every monday',
  'add a reminder to check the oven in 20 minutes',
  'make a reminder for passport renewal next week',
  'remind me to book flights next month',
  'set me a reminder to call the bank at 4',
  'reminder about the launch checklist tomorrow',
  'nudge me to stand up in an hour',
  "don't let me forget the tickets tonight",
] as const;

describe('curated intent routing set', () => {
  test('routes at least 50 edge-case phrases to none', () => {
    assert.ok(noneCases.length >= 50);
    for (const input of noneCases) {
      const result = classifyIntent(input);
      assert.equal(result.intent, 'none', input);
    }
  });

  test('keeps representative task phrases on task', () => {
    for (const input of taskCases) {
      const result = classifyIntent(input);
      assert.equal(result.intent, 'task', input);
    }
  });

  test('keeps representative calendar phrases on calendar', () => {
    for (const input of calendarCases) {
      const result = classifyIntent(input);
      assert.equal(result.intent, 'calendar', input);
    }
  });

  test('keeps representative reminder phrases on reminder', () => {
    for (const input of reminderCases) {
      const result = classifyIntent(input);
      assert.equal(result.intent, 'reminder', input);
    }
  });
});
