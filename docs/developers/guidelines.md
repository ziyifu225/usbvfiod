# Code Guidelines

This document lists code design and style choices that developers
should adhere to when modifying or extending the source code.

## DMA Address Calculations 

When calculating DMA addresses to access guest memory, you should use
wrap-around calculations such as `u64::wrapping_add`.

Overflows in addresses calculations should not occur during normal
operation, only when the USB driver in the guest wants to shoot
itself in the foot. In such cases, we want to behave like a physical
XHCI controller and wrap around.

Explicitly treating the calculations as wrapping aligns the behavior between debug and release builds.
