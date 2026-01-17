import { motion } from "framer-motion";
import { Children, cloneElement, isValidElement, ReactNode } from "react";
import { cn } from "../../utils/cn";

export interface AnimatedListProps {
  children: ReactNode;
  className?: string;
  staggerDelay?: number;
  animation?: "fadeIn" | "slideUp" | "slideLeft";
  itemClassName?: string;
}

export default function AnimatedList({
  children,
  className,
  staggerDelay = 0.05,
  animation = "slideUp",
  itemClassName,
}: AnimatedListProps) {
  const items = Children.toArray(children);

  const variants = {
    fadeIn: {
      hidden: { opacity: 0 },
      visible: (i: number) => ({
        opacity: 1,
        transition: {
          delay: i * staggerDelay,
          duration: 0.3,
        },
      }),
    },
    slideUp: {
      hidden: { opacity: 0, y: 20 },
      visible: (i: number) => ({
        opacity: 1,
        y: 0,
        transition: {
          delay: i * staggerDelay,
          duration: 0.4,
        },
      }),
    },
    slideLeft: {
      hidden: { opacity: 0, x: 20 },
      visible: (i: number) => ({
        opacity: 1,
        x: 0,
        transition: {
          delay: i * staggerDelay,
          duration: 0.4,
        },
      }),
    },
  };

  const variant = variants[animation];

  return (
    <div className={cn(className)}>
      {items.map((child, index) => {
        if (!isValidElement(child)) return child;

        return (
          <motion.div
            key={index}
            custom={index}
            initial="hidden"
            animate="visible"
            variants={variant}
            className={itemClassName}
          >
            {cloneElement(child)}
          </motion.div>
        );
      })}
    </div>
  );
}
