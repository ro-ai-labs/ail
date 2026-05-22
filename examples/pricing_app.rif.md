app Pricing

things:
  thing Product
    field id: Id<Product>
    field price: Decimal
    field discount_rate: Decimal
    field final_price: Decimal

operations:
  operation Pricing.apply(rate: Decimal) -> Unit

intent PriceProduct

subject:
  product: Product

steps:
  1. Set price
     set: product.price = 12.50
     changes: product.price

  2. Set discount rate
     set: product.discount_rate = 0.25
     changes: product.discount_rate

  3. Apply rate
     call: Pricing.apply(0.25)

  4. Compute final price
     compute: product.final_price = product.price-product.price*product.discount_rate
     changes: product.final_price

guarantees:
  if this intent succeeds:
    product.price == 12.50
