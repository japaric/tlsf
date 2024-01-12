#!/usr/bin/env python3

from matplotlib import pyplot as plt
import numpy as np
import seaborn as sns

BINS = 30
plt.rcParams['font.family'] = 'monospace'

# op = 'malloc'
op = 'memalign'

memalign_ok = np.loadtxt(op + '.ok')
memalign_fail = np.loadtxt(op + '.fail')
free = np.loadtxt('free')

memalign_all = np.concatenate((memalign_ok, memalign_fail), axis=0)

fig, axes = plt.subplots(nrows=2, ncols=2, figsize=(12, 6))

def plot(data, title, ax):
    sns.histplot(data=data, kde=True, bins=BINS, ax=ax)
    ax.title.set_text(title)
    ax.set_xlabel('Clock cycles')

    print(title, np.min(data), np.max(data))

plot(memalign_all, op +  ' (ALL)', axes[0][0])
plot(memalign_fail, op + ' (FAIL)', axes[0][1])
plot(memalign_ok, op + ' (OK)', axes[1][0])
plot(free, 'free', axes[1][1])

fig.tight_layout()
plt.subplots_adjust(left=0.05, right=0.95, top=0.95, hspace=0.35)
plt.savefig(op + '-histograms.svg')
