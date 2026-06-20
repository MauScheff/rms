# Payment Lifecycle

```text
requested -> authorized -> captured -> partially-refunded -> refunded
requested -> declined
authorized -> expired
```

The payments module owns lifecycle decisions and rejects illegal transitions before provider effects are attempted.

