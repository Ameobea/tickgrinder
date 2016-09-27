"use strict";
/*jslint node: true */
/*jslint browser: true*/ /*global Highcharts, $*/

// var drawTrades = function(pair, range){
//   var params = {pair: pair};

//   getData(pair, "tradeHistory", params, range, function(tradeData){
//     var filtered = tradeData.filter(function(trade){
//       return trade.pair == pair;
//     });
//     tradeIterator(filtered);
//   });
// };

// var tradeIterator = function(trades){
//   if(trades.length > 0){
//     var tradeStart = trades[0];
//     trades.shift();

//     if(trades.length > 0){
//       var tradeStop = trades[0];
//       trades.shift();
//       drawTradeLine(tradeStart.timestamp, tradeStop.timestamp, tradeStart.openPrice, tradeStop.closePrice, tradeStart.direction);

//       tradeIterator(trades);
//     }else{
//       //TODO: draw trade line to current price
//     }
//   }
// };

// var drawTradeLine = function(startTime, stopTime, startPrice, stopPrice, direction){
//   var color;
//   if(direction){
//     color = "blue";
//   }else{
//     color = "red";
//   }

//   var series = {yAxis: "priceAverageAxis", showInLegend: false, type: "line", color: color};
//   series.data = [[startTime * 1000, startPrice], [stopTime * 1000, stopPrice]];

//   mainGraph.addSeries(series);
// };

/// Holds references to all open charts
var openCharts = [];

/// Appends a new chart to the bottom of the page and returns a reference to its DOM object
function createChart() {
  var html = $("#charts").html();
  var i = openCharts.length;
  var chart_code = `<div id="chart_${i}_wrapper"><div id="chart_${i}"></div>
    <center><br><button id="chart_${i}_delete">Delete Chart</button></center>
    </div>`;
  html += chart_code;
  $("#charts").html(html);

  // set up chart deletion listener
  $(`#chart_${i}_delete`).click(()=>{
    $(`#chart_${i}_wrapper`).remove();
    openCharts[i] = null;
  });

  var new_chart = new Highcharts.Chart({
    chart: {
      renderTo: `chart_${i}`,
      zoomType: "x"
    },
    xAxis: {
      type: 'datetime',
      tickPixelInterval: 150,
      maxZoom: 20 * 1000
    },
    plotOptions: {
      series: {
        animation: false
      }
    }
  });

  openCharts.push(new_chart);
  return new_chart;
}

/// Clear all plotted data from all open charts
var clearChart = chart=>{
  for(var i=0;i<openCharts.length;i++){
    while(openCharts[i].series.length > 0)
      openCharts[i].series[0].remove(true);
  }
};

/// Parses the supplied macro and adds a new graph to the bottom of the page.
function loadMacro(macroString) {
  if(macroString[0] == "{"){
    var macro = JSON.parse(macroString);
  }else{
    // TODO: Load saved macro string from database based on macro code
  }

  // make request to macro data endpoint
  $.get(`../data/${macro.indicator}/${macro.symbol}/${macro.startTime}` +
      `/${macro.endTime}/${JSON.stringify(macro.data)}`, res=>{
    console.log(res);
    var new_chart = createChart();
    new_chart.setTitle(macro.chart_title || "Unnamed Chart");
    matchPlot(macro.indicator, new_chart, res);
  });
}

/// Generates a macro string based on the supplied inputs
function generateMacro() {
  var indicator = $("#indicator").val();
  var symbol = $("#pairInput").val();
  var timeRange = $("#timeRange").val();

  var macro = {};
  macro.indicator = indicator;
  macro.symbol = symbol;
  macro.startTime = Date.now() - (timeRange*1000);
  macro.endTime = Date.now();
  macro.data = {};

  return JSON.stringify(macro);
}

/// initialize listeners for all of the buttons
var setupConfigListeners = ()=>{
  $("#generateMacro").off().click(()=>{
    var macroString = generateMacro();
    $("#generatedMacro").val(macroString);
  });

  $("#loadMacro").off().click(()=>{
    loadMacro($("#macroInput").val());
  });
};

$(document).ready(()=>{
  setupConfigListeners();
});
