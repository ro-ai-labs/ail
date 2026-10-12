# Conway Life library

Import `ail.std.life` 0.1.0 as `Life`. The pure APIs are
`step(width:Int,height:Int,cells:List<Int>)->List<Int>`,
`evolve(width:Int,height:Int,cells:List<Int>,generations:Int)->List<Int>`,
`population(cells:List<Int>)->Int`, and `runRequest(text:Text)->Json`.

Grids are positive rectangular dimensions, row-major bits, with dead exterior
and simultaneous B3/S23 transitions. Results are immutable and canonical.
Invalid well-typed grids/bits or negative direct generations raise VALUE.
Zero generations still validates every cell. Population accepts an empty list.

Requests require unambiguous strict integer width, height, generations and flat
integer cells. Error priority is REQUEST, GRID, GENERATIONS. Success contains
ordered ok, cells, population, error fields; failures use empty cells and zero
population. Fatal runtime limits propagate. No arbitrary simulation cap is added.
The existing finite fuel, 65,536-item, 1 MiB value and 64 MiB arena limits apply.
