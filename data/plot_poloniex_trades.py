import pandas as pd
import matplotlib.pyplot as plt
import math

df = pd.read_csv("polo_book_rew_trade.csv")
df.columns = ['timestamp', 'tradeID', 'rate', 'amount', 'date', 'total', 'isBuy']
ax = plt.gca()
ax.scatter(x=df['timestamp'], y=df['rate'], marker='o', c='b', s=df['amount'].map(lambda x: math.sqrt(x * 100)))

ax.set_autoscaley_on(False)
ax.set_ylim([min(df['rate']), max(df['rate'])])

plt.show()

